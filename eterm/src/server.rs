use crate::{
    messages::{into_clipped_net_meshes, ClippedNetMesh},
    ClientToServerMessage, ServerToClientMessage,
};
use anyhow::Context as _;
use egui::RawInput;
use std::{
    collections::HashMap,
    net::{SocketAddr, TcpListener},
    time::{Duration, Instant},
};

// Respond to user input with a maximum 60 frames per second
pub const DEFAULT_MAX_UPDATE_INTERVAL: Duration = Duration::from_millis(1000 / 60);
// Send at least 1 frame per second
pub const DEFAULT_MIN_UPDATE_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ClientId(u64);

pub struct Server {
    next_client_id: u64,
    tcp_listener: TcpListener,
    clients: HashMap<SocketAddr, Client>,
    minimum_update_interval: Duration,
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
            minimum_update_interval: DEFAULT_MIN_UPDATE_INTERVAL,
        })
    }

    /// Send a new frame to each client at least this often.
    /// Default: one second.
    pub fn set_minimum_update_interval(&mut self, minimum_update_interval: Duration) {
        self.minimum_update_interval = minimum_update_interval;
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
                            //prev_input: None,
                            last_client_time: None,
                            last_update: Instant::now() - DEFAULT_MIN_UPDATE_INTERVAL,
                            last_visuals: Default::default(),
                            max_update_interval: DEFAULT_MAX_UPDATE_INTERVAL,
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
    /// The client's time of the last input.
    last_client_time: Option<f64>,
    last_update: std::time::Instant,
    last_visuals: Vec<ClippedNetMesh>,
    max_update_interval: Duration,
}

impl Client {
    fn disconnect(&mut self) {
        self.tcp_endpoint = None;
        self.last_visuals = Default::default();
    }

    // Show is called from the app's main loop (e.g. 60 time per sec),
    // but new frames are only build and send to the eterm client every
    // Client.max_update_interval or less when there is no new input.
    // Input sent by the client is continously collected in the backgound
    // and kept in Client.new_input. No input is lost, even if the
    // max_update_interval is set to a high number.
    fn show(
        &mut self,
        do_ui: &mut dyn FnMut(&egui::Context, ClientId),
        minimum_update_interval: Duration,
    ) {
        // Don't do anything if there is no client
        if self.tcp_endpoint.is_none() {
            return;
        }

        let minimum_interval_has_passed = self.last_update.elapsed() >= minimum_update_interval;
        let input_triggered_update =
            self.new_input.is_some() && self.last_update.elapsed() >= self.max_update_interval;

        if minimum_interval_has_passed || input_triggered_update {
            let message = self.create_frame(do_ui);
            self.send_message(&message);
        }
    }

    // Create a frame for the client
    fn create_frame(
        &mut self,
        do_ui: &mut dyn FnMut(&egui::Context, ClientId),
    ) -> ServerToClientMessage {
        // Reset instant of last update
        self.last_update = Instant::now();

        // Take accumulated input
        let mut input = self.new_input.take().unwrap_or_default();

        // Override client time with server time
        input.time = Some(self.start_time.elapsed().as_secs_f64());

        // Refresh egui
        let full_output = self
            .egui_ctx
            .run(input, |egui_ctx| do_ui(egui_ctx, self.client_id));

        // tesselate shapes
        let clipped_primitives = self.egui_ctx.tessellate(full_output.clone().shapes);
        let clipped_net_mesh = into_clipped_net_meshes(clipped_primitives);
        let textures_delta = full_output.textures_delta.clone();

        // Prepare a new frame for the client
        let frame_index = self.frame_index;
        self.frame_index += 1;

        crate::ServerToClientMessage::Frame {
            frame_index,
            platform_output: full_output.platform_output,
            clipped_net_mesh,
            textures_delta,
            client_time: self.last_client_time.take(),
        }
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
                    //points_per_pixel,
                } => {
                    //eprintln!("{:?}", raw_input);
                    self.append_input(raw_input);
                    self.last_client_time = Some(client_time);
                    //self.points_per_pixel = points_per_pixel;
                    // keep polling for more messages
                }
                ClientToServerMessage::Goodbye => {
                    self.disconnect();
                    return;
                }
            }
        }
    }

    // accumulates input from the client
    fn append_input(&mut self, new_input: RawInput) {
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
