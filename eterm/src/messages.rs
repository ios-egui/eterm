use egui::{
    epaint::{self, Color32, Pos2, Primitive, Rect, TextureId},
    ClippedPrimitive,
};
use serde::{Deserialize, Serialize};

/// Optimized for transport over a network.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClippedNetMesh {
    pub clip_rect: Rect,
    pub mesh: NetMesh,
}

pub fn into_clipped_net_meshes(primitives: Vec<ClippedPrimitive>) -> Vec<ClippedNetMesh> {
    primitives
        .into_iter()
        .filter_map(to_clipped_net_mesh)
        .collect()
}

pub fn to_clipped_net_mesh(p: ClippedPrimitive) -> Option<ClippedNetMesh> {
    if let Primitive::Mesh(m) = p.primitive {
        Some(ClippedNetMesh {
            clip_rect: p.clip_rect,
            mesh: (&m).into(),
        })
    } else {
        None
    }
}

/// Like [`epaint::Mesh`], but optimized for transport over a network.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NetMesh {
    pub texture_id: TextureId,
    pub indices: Vec<u32>,
    pub pos: Vec<Pos2>,
    pub uv: Vec<Pos2>,
    pub color: Vec<Color32>,
}

impl From<&epaint::Mesh> for NetMesh {
    fn from(mesh: &epaint::Mesh) -> Self {
        Self {
            texture_id: mesh.texture_id,
            indices: mesh.indices.clone(),
            pos: mesh.vertices.iter().map(|v| v.pos).collect(),
            uv: mesh.vertices.iter().map(|v| v.uv).collect(),
            color: mesh.vertices.iter().map(|v| v.color).collect(),
        }
    }
}

impl From<&NetMesh> for epaint::Mesh {
    fn from(mesh: &NetMesh) -> epaint::Mesh {
        epaint::Mesh {
            texture_id: mesh.texture_id,
            indices: mesh.indices.clone(),
            vertices: itertools::izip!(&mesh.pos, &mesh.uv, &mesh.color)
                .map(|(&pos, &uv, &color)| epaint::Vertex { pos, uv, color })
                .collect(),
        }
    }
}
