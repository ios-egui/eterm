use crate::{
    messages::{into_clipped_net_meshes, ClippedNetMesh},
    ClientToServerMessage,
};
use anyhow::Context as _;
use egui::{epaint::Primitive, ClippedPrimitive, Mesh, RawInput};
use std::{
    collections::HashMap,
    net::{SocketAddr, TcpListener},
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ClientId(u64);

pub struct Server {
    next_client_id: u64,
    tcp_listener: TcpListener,
    clients: HashMap<SocketAddr, Client>,
    minimum_update_interval: f32,
}

impl Server {
    /// Start listening for connections on this addr (e.g. "0.0.0.0:8585")
    ///
    /// # Errors
    /// Can fail if the port is already taken.
    pub fn new(bind_addr: &str) -> anyhow::Result<Self> {
        let tcp_listener = TcpListener::bind(bind_addr).context("binding server TCP socket")?;
        tcp_listener
            .set_nonblocking(true)
            .context("TCP set_nonblocking")?;

        Ok(Self {
            next_client_id: 0,
            tcp_listener,
            clients: Default::default(),
            minimum_update_interval: 1.0,
        })
    }

    /// Send a new frame to each client at least this often.
    /// Default: one second.
    pub fn set_minimum_update_interval(&mut self, seconds: f32) {
        self.minimum_update_interval = seconds;
    }

    /// Call frequently (e.g. 60 times per second) with the ui you'd like to show to clients.
    ///
    /// # Errors
    /// Underlying TCP errors.
    pub fn show(&mut self, mut do_ui: impl FnMut(&egui::Context, ClientId)) -> anyhow::Result<()> {
        self.show_dyn(&mut do_ui)
    }

    fn show_dyn(&mut self, do_ui: &mut dyn FnMut(&egui::Context, ClientId)) -> anyhow::Result<()> {
        self.accept_new_clients()?;
        self.try_receive();

        for client in self.clients.values_mut() {
            client.show(do_ui, self.minimum_update_interval);
        }
        Ok(())
    }

    /// non-blocking
    fn accept_new_clients(&mut self) -> anyhow::Result<()> {
        loop {
            match self.tcp_listener.accept() {
                Ok((tcp_stream, client_addr)) => {
                    tcp_stream
                        .set_nonblocking(true)
                        .context("stream.set_nonblocking")?;
                    let tcp_endpoint = crate::TcpEndpoint { tcp_stream };

                    // reuse existing client - especially the egui context
                    // which contains things like window positons:
                    let clients = &mut self.clients;
                    let next_client_id = &mut self.next_client_id;
                    let client = clients.entry(client_addr).or_insert_with(|| {
                        let client_id = ClientId(*next_client_id);
                        *next_client_id += 1;

                        Client {
                            client_id,
                            addr: client_addr,
                            tcp_endpoint: None,
                            start_time: std::time::Instant::now(),
                            frame_index: 0,
                            egui_ctx: Default::default(),
                            new_input: None,
                            prev_input: None,
                            client_time: None,
                            last_update: None,
                            last_visuals: Default::default(),
                        }
                    });

                    client.tcp_endpoint = Some(tcp_endpoint);

                    tracing::info!("{} connected", client.info());
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    break; // No (more) new clients
                }
                Err(err) => {
                    anyhow::bail!("eterm server TCP error: {:?}", err);
                }
            }
        }
        Ok(())
    }

    /// non-blocking
    fn try_receive(&mut self) {
        for client in self.clients.values_mut() {
            client.try_receive();
        }
    }
}

// ----------------------------------------------------------------------------

struct Client {
    client_id: ClientId,
    addr: SocketAddr,
    tcp_endpoint: Option<crate::TcpEndpoint>,
    start_time: std::time::Instant,
    frame_index: u64,
    egui_ctx: egui::Context,
    /// Set when there is something to do. Cleared after painting.
    new_input: Option<egui::RawInput>,
    prev_input: Option<egui::RawInput>,
    /// The client time of the last input we got from them.
    client_time: Option<f64>,
    last_update: Option<std::time::Instant>,
    last_visuals: Vec<ClippedNetMesh>,
}

impl Client {
    fn disconnect(&mut self) {
        self.tcp_endpoint = None;
        self.last_visuals = Default::default();
    }

    fn show(
        &mut self,
        do_ui: &mut dyn FnMut(&egui::Context, ClientId),
        minimum_update_interval: f32,
    ) {
        // don't do anything if there are no clients
        if self.tcp_endpoint.is_none() {
            return;
        }

        // Take most recently received client time (from Client)
        let client_time = self.client_time.take();

        // Time since last update
        let time_since_last_update = if let Some(instant) = self.last_update {
            instant.elapsed().as_secs_f32()
        } else {
            f32::INFINITY
        };

        // Are we forced to update? Or can we wait?
        let can_wait = time_since_last_update < minimum_update_interval;

        if self.new_input.is_none() {
            // no new user input available - skip this iteration if we can wait
            if can_wait {
                //eprint!("skip, ");
                return;
            } else {
                //eprint!("refresh event");
                return;
            }
        } else {
            //eprint!("input event");
        }
        //self.prev_input = self.new_input.clone();

        // Use cached input from client if available
        let mut input = self.new_input.take().unwrap();
        input.pixels_per_point = Some(2.0);
        //eprintln!("Received input: {:?}", input);

        // Reset instant of last update
        self.last_update = Some(std::time::Instant::now());

        // Override client time with server time
        input.time = Some(self.start_time.elapsed().as_secs_f64());

        // refresh egui
        let full_output = self
            .egui_ctx
            .run(input, |egui_ctx| do_ui(egui_ctx, self.client_id));

        // convert shapes to network friendly shapes
        //let clipped_shapes = &full_output.shapes;
        //let clipped_net_shapes = crate::net_shape::to_clipped_net_shapes(clipped_shapes.clone());

        // keep a copy of repaint_after - we will need it after full_ouput is out of scope
        let repaint_after = full_output.repaint_after;

        // prepare a new frame for the client
        let frame_index = self.frame_index;
        self.frame_index += 1;
        //eprintln!("Send frame {frame_index}");

        // create a new glium
        //use glium::glutin;
        //let event_loop = glutin::event_loop::EventLoop::with_user_event();
        //let ev = EventLoopBuilder::<T>::with_user_event().build();
        //let display = create_display(&event_loop);
        //let mut egui_glium = egui_glium::EguiGlium::new(&display, &event_loop);

        let clipped_primitives = self.egui_ctx.tessellate(full_output.clone().shapes);
        let clipped_net_mesh = into_clipped_net_meshes(clipped_primitives);
        let textures_delta = full_output.textures_delta.clone();

        let message = crate::ServerToClientMessage::Frame {
            frame_index,
            platform_output: full_output.platform_output,
            clipped_net_mesh: clipped_net_mesh.clone(),
            //full_output,
            textures_delta,
            // replace clipped_net_shapes with meshes,
            //clipped_net_shapes: clipped_primitives.clone(),
            client_time,
        };

        self.last_visuals = clipped_net_mesh;
        self.send_message(&message);

        // if self.last_update.is_none() || self.last_update.unwrap().elapsed() > repaint_after {
        //     //eprintln!("frame {} painted, needs_repaint", self.frame_index);
        //     // Reschedule asap (don't wait for client) to request it.
        //     self.new_input = Some(Default::default());
        // } else {
        //     //eprintln!("frame {} painted", self.frame_index);
        // }
    }

    fn info(&self) -> String {
        format!("Client {} ({})", self.client_id.0, self.addr)
    }

    fn send_message(&mut self, message: &impl serde::Serialize) {
        if let Some(tcp_endpoint) = &mut self.tcp_endpoint {
            match tcp_endpoint.send_message(&message) {
                Ok(()) => {}
                Err(err) => {
                    tracing::error!(
                        "Failed to send to client {:?} {}: {:?}. Disconnecting.",
                        self.client_id,
                        self.addr,
                        crate::error_display_chain(err.as_ref())
                    );
                    self.disconnect();
                }
            }
        }
    }

    /// non-blocking
    fn try_receive(&mut self) {
        loop {
            let tcp_endpoint = match &mut self.tcp_endpoint {
                Some(tcp_endpoint) => tcp_endpoint,
                None => return,
            };

            let message = match tcp_endpoint.try_receive_message() {
                Ok(None) => {
                    return;
                }
                Ok(Some(message)) => message,
                Err(err) => {
                    tracing::error!(
                        "Failed to read from client {}: {:?}. Disconnecting.",
                        self.info(),
                        crate::error_display_chain(err.as_ref())
                    );
                    self.disconnect();
                    return;
                }
            };

            match message {
                ClientToServerMessage::Input {
                    raw_input,
                    client_time,
                } => {
                    //eprintln!("{:?}", raw_input);
                    self.input(raw_input);
                    self.client_time = Some(client_time);
                    // keep polling for more messages
                }
                ClientToServerMessage::Goodbye => {
                    self.disconnect();
                    return;
                }
            }
        }
    }

    fn input(&mut self, new_input: RawInput) {
        match &mut self.new_input {
            None => {
                self.new_input = Some(new_input);
            }
            Some(existing_input) => {
                existing_input.append(new_input);
            }
        }
    }
}
