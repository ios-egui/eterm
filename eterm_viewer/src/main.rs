#![forbid(unsafe_code)]
#![warn(
    clippy::all,
    clippy::await_holding_lock,
    clippy::char_lit_as_u8,
    clippy::checked_conversions,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::doc_markdown,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::exit,
    clippy::expl_impl_clone_on_copy,
    clippy::explicit_deref_methods,
    clippy::explicit_into_iter_loop,
    clippy::fallible_impl_from,
    clippy::filter_map_next,
    clippy::float_cmp_const,
    clippy::fn_params_excessive_bools,
    clippy::if_let_mutex,
    clippy::imprecise_flops,
    clippy::inefficient_to_string,
    clippy::invalid_upcast_comparisons,
    clippy::large_types_passed_by_value,
    clippy::let_unit_value,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::manual_ok_or,
    clippy::map_err_ignore,
    clippy::map_flatten,
    clippy::match_on_vec_items,
    clippy::match_same_arms,
    clippy::match_wildcard_for_single_variants,
    clippy::mem_forget,
    clippy::mismatched_target_os,
    clippy::missing_errors_doc,
    clippy::missing_safety_doc,
    clippy::mut_mut,
    clippy::mutex_integer,
    clippy::needless_borrow,
    clippy::needless_continue,
    clippy::needless_pass_by_value,
    clippy::option_option,
    clippy::path_buf_push_overwrite,
    clippy::ptr_as_ptr,
    clippy::ref_option_ref,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_functions_in_if_condition,
    clippy::string_add_assign,
    clippy::string_add,
    clippy::string_lit_as_bytes,
    clippy::string_to_string,
    clippy::todo,
    clippy::trait_duplication_in_bounds,
    clippy::unimplemented,
    clippy::unnested_or_patterns,
    clippy::unused_self,
    clippy::useless_transmute,
    clippy::verbose_file_reads,
    clippy::zero_sized_map_values,
    future_incompatible,
    missing_crate_level_docs,
    nonstandard_style,
    rust_2018_idioms
)]
#![allow(clippy::float_cmp)]
#![allow(clippy::manual_range_contains)]
mod test;

use std::time::Duration;

use egui::{
    epaint::{textures, Primitive},
    ClippedPrimitive, FullOutput, Mesh, TexturesDelta,
};
use eterm::{messages::ClippedNetMesh, EtermFrame};
use glium::glutin;

/// We reserve this much space for eterm to show some stats.
/// The rest is used for the view of the remove server.
const TOP_BAR_HEIGHT: f32 = 0.0; //24.0;

/// Repaint every so often to check connection status etc.
const MIN_REPAINT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

/// eterm viewer viewer.
///
/// Connects to an eterm server somewhere.
#[derive(argh::FromArgs)]
struct Arguments {
    /// which server to connect to, e.g. `127.0.0.1:8505`.
    #[argh(option)]
    url: String,
}

fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let opt: Arguments = argh::from_env();
    let mut client = eterm::Client::new(opt.url);

    let event_loop = glutin::event_loop::EventLoop::with_user_event();
    let display = create_display(&event_loop);

    let mut egui_glium = egui_glium::EguiGlium::new(&display, &event_loop);

    let mut last_sent_input = None;

    let mut last_received_clipped_new_mesh: Vec<ClippedNetMesh> = vec![];
    let mut last_received_texture_delta = Default::default();
    let mut last_received_frame_index = 0;
    let mut last_received_full_output = FullOutput::default();

    let mut last_clipped_primitives = vec![];
    let mut local_repaint_after = Duration::from_secs(5);
    let received_repaint_after = Duration::from_millis(5);

    let mut last_repaint = std::time::Instant::now();

    let pixels_per_point = 2.0; //egui_glium.egui_winit.pixels_per_point();

    // work_arround_to_init_fonts
    {
        client.update(pixels_per_point);

        let raw_input = egui_glium
            .egui_winit
            .take_egui_input(display.gl_window().window());
        client.send_input(raw_input.clone());

        let _ = egui_glium.egui_ctx.run(raw_input, |egui_ctx| {
            egui::SidePanel::left("").show(egui_ctx, |_| {});
        });
    }

    event_loop.run(move |event, _, control_flow| {
        let mut redraw = || {
            // get input from viewer
            let raw_input = egui_glium
                .egui_winit
                .take_egui_input(display.gl_window().window());

            // send input to server
            let input_changed = last_sent_input.as_ref() != Some(&raw_input);
            if input_changed {
                client.send_input(raw_input.clone());
                last_sent_input = Some(raw_input.clone());
            }

            // check if server has sent something to draw
            let new_frame_option = client.update(pixels_per_point);

            // if there is a new frame, paint it
            if let Some(frame) = new_frame_option {
                let EtermFrame {
                    frame_index,
                    platform_output,
                    clipped_net_mesh,
                    textures_delta,
                } = frame;

                egui_glium.egui_winit.handle_platform_output(
                    display.gl_window().window(),
                    &egui_glium.egui_ctx,
                    platform_output,
                );

                // save the server primitives for use later on
                last_received_frame_index = frame_index;
                last_received_clipped_new_mesh = clipped_net_mesh;
                last_received_texture_delta = textures_delta;

                if true
                    || last_repaint.elapsed() >= received_repaint_after
                    || last_repaint.elapsed() >= MIN_REPAINT_INTERVAL
                    || last_repaint.elapsed() >= local_repaint_after
                {
                    // paint the server ui - with the last received primitives
                    use glium::Surface as _;
                    let mut target = display.draw();

                    let cc = egui::Rgba::from_rgb(0.5, 0.3, 0.2);
                    target.clear_color(cc[0], cc[1], cc[2], cc[3]);

                    let clipped_primitives =
                        into_clipped_primitives(last_received_clipped_new_mesh.clone());

                    last_clipped_primitives = clipped_primitives.clone();

                    egui_glium.painter.paint_and_update_textures(
                        &display,
                        &mut target,
                        pixels_per_point,
                        &clipped_primitives,
                        &last_received_texture_delta,
                    );

                    target.finish().unwrap();
                }
            }

            display.gl_window().window().request_redraw();
            *control_flow = glutin::event_loop::ControlFlow::Wait;
            std::thread::sleep(std::time::Duration::from_millis(10));
        };

        match event {
            // Platform-dependent event handlers to workaround a winit bug
            // See: https://github.com/rust-windowing/winit/issues/987
            // See: https://github.com/rust-windowing/winit/issues/1619
            glutin::event::Event::RedrawEventsCleared if cfg!(windows) => redraw(),
            glutin::event::Event::RedrawRequested(_) if !cfg!(windows) => redraw(),

            glutin::event::Event::WindowEvent { event, .. } => {
                use glutin::event::WindowEvent;
                if matches!(event, WindowEvent::CloseRequested | WindowEvent::Destroyed) {
                    *control_flow = glium::glutin::event_loop::ControlFlow::Exit;
                }

                let _ = egui_glium.on_event(&event);

                display.gl_window().window().request_redraw();
            }

            _ => (),
        }
    });
}

fn create_display(event_loop: &glutin::event_loop::EventLoop<()>) -> glium::Display {
    let window_builder = glutin::window::WindowBuilder::new()
        .with_resizable(true)
        .with_inner_size(glutin::dpi::LogicalSize {
            width: 800.0,
            height: 600.0,
        })
        .with_title("eterm viewer");

    let context_builder = glutin::ContextBuilder::new()
        .with_depth_buffer(0)
        .with_double_buffer(Some(true))
        .with_srgb(true)
        .with_stencil_buffer(0)
        .with_vsync(true);

    glium::Display::new(window_builder, context_builder, event_loop).unwrap()
}
fn into_clipped_primitives(meshes: Vec<ClippedNetMesh>) -> Vec<ClippedPrimitive> {
    meshes.into_iter().map(to_clipped_primitve).collect()
}

fn to_clipped_primitve(m: ClippedNetMesh) -> ClippedPrimitive {
    ClippedPrimitive {
        clip_rect: m.clip_rect,
        primitive: Primitive::Mesh(Mesh::from(&m.mesh)),
    }
}
