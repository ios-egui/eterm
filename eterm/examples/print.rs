// //! Print info to help guide what encoding to use for the network.

// /// `anti_alias=false` gives us around 23% savings in final bandwidth
// fn example_output(anti_alias: bool) -> (egui::FullOutput, Vec<egui::ClippedPrimitive>) {
//     let ctx = egui::Context::default();

//     let raw_input = egui::RawInput::default();
//     let mut demo_windows = egui_demo_lib::DemoWindows::default();
//     let output = ctx.run(raw_input, |ctx| demo_windows.ui(ctx));
//     let clipped_meshes = ctx.tessellate(output.clone().shapes);
//     (output, clipped_meshes)
// }

// fn _example_shapes() -> egui::FullOutput {
//     let mut ctx = egui::Context::default();
//     let raw_input = egui::RawInput::default();
//     let mut demo_windows = egui_demo_lib::DemoWindows::default();
//     ctx.run(raw_input, |ctx| demo_windows.ui(ctx))
// }

// fn bincode<S: ?Sized + serde::Serialize>(data: &S) -> Vec<u8> {
//     use bincode::Options as _;
//     bincode::options().serialize(data).unwrap()
// }

// fn zstd(data: &[u8], level: i32) -> Vec<u8> {
//     zstd::encode_all(std::io::Cursor::new(data), level).unwrap()
// }

// fn zstd_kb(data: &[u8], level: i32) -> f32 {
//     zstd(data, level).len() as f32 * 1e-3
// }

// // ----------------------------------------------------------------------------

// fn print_encodings<S: ?Sized + serde::Serialize>(data: &S) {
//     let encoded = bincode(data);
//     println!("bincode: {:>6.2} kB", encoded.len() as f32 * 1e-3);
//     println!("zstd-0:  {:>6.2} kB", zstd_kb(&encoded, 0));
//     println!("zstd-5:  {:>6.2} kB", zstd_kb(&encoded, 5));
//     // println!("zstd-15: {:>6.2} kB", zstd_kb(&encoded, 15));
//     // println!("zstd-21: {:>6.2} kB (too slow)", zstd_kb(&encoded, 21)); // way too slow
// }

// fn print_compressions(clipped_meshes: &[egui::ClippedPrimitive]) {
//     let mut num_vertices = 0;
//     let mut num_indices = 0;
//     let mut bytes_vertices = 0;
//     let mut bytes_indices = 0;
//     for egui::ClippedPrimitive {
//         clip_rect,
//         primitive,
//     } in clipped_meshes
//     {
//         // num_vertices += primitive.vertices.len();
//         // num_indices += primitive.indices.len();
//         // bytes_vertices += primitive.vertices.len() * std::mem::size_of_val(&primitive.vertices[0]);
//         // bytes_indices += primitive.indices.len() * std::mem::size_of_val(&primitive.indices[0]);
//     }
//     //     let mesh_bytes = bytes_indices + bytes_vertices;
//     //     println!(
//     //         "vertices: {:>5}  {:>6.2} kb",
//     //         num_vertices,
//     //         bytes_vertices as f32 * 1e-3
//     //     );
//     //     println!(
//     //         "indices:  {:>5}  {:>6.2} kb",
//     //         num_indices,
//     //         bytes_indices as f32 * 1e-3
//     //     );
//     //     println!();

//     //     println!("raw:     {:>6.2} kB", mesh_bytes as f32 * 1e-3);
//     //     println!();
//     //     print_encodings(&clipped_meshes);
//     //     println!();
//     //     println!("Flattened mesh:");
//     //     print_encodings(&net_meshes);
//     //     println!();
//     //     println!("Quantized positions:");
//     //     print_encodings(&quantized_meshes);

//     //     // Other things I've tried: delta-encoded positions (5-10% worse).
// }

fn main() {
    //     println!("FontDefinitions:");
    //     let font_definitions = egui::FontDefinitions::default();
    //     print_encodings(&font_definitions);
    //     println!();

    //     let (_, clipped_meshes) = example_output(true);
    //     println!("Antialiasing ON:");
    //     print_compressions(&clipped_meshes);
    //     println!();

    //     let (_, clipped_meshes) = example_output(false);
    //     println!("Antialiasing OFF:");
    //     print_compressions(&clipped_meshes);
    //     println!();
}

// fn quantize(f: f32) -> f32 {
//     // TODO: should be based on pixels_to_point

//     // let precision = 2.0; // 15% wins
//     let precision = 8.0; // 12% wins

//     (f * precision).round() / precision
// }
