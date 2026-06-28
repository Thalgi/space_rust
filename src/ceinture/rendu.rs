use super::Ceinture;
use crate::astre::CameraInfo;
use macroquad::models::{draw_mesh, Mesh, Vertex};
use macroquad::prelude::*;

const QUADS_PAR_LOT: usize = 400; // billboards par draw call (évite le clamp)

impl Ceinture {
    pub(super) fn dessiner(&mut self, cam: &CameraInfo) {
        self.verts.clear();
        self.inds.clear();
        let mut quads = 0;

        for it in &self.items {
            let pos = it.a1 * (it.r * it.angle.cos()) + it.q * (it.r * it.angle.sin());
            let c = it.couleur;
            let col = Color::new(c.x, c.y, c.z, 1.0);
            let i0 = self.verts.len() as u16;
            for cn in &it.coins {
                let pt = pos + cam.right * cn.x + cam.up * cn.y;
                self.verts.push(Vertex::new2(pt, vec2(0.0, 0.0), col));
            }
            self.inds
                .extend_from_slice(&[i0, i0 + 1, i0 + 2, i0, i0 + 2, i0 + 3]);

            quads += 1;
            if quads >= QUADS_PAR_LOT {
                flush(&mut self.verts, &mut self.inds);
                quads = 0;
            }
        }
        flush(&mut self.verts, &mut self.inds);
    }
}

fn flush(verts: &mut Vec<Vertex>, inds: &mut Vec<u16>) {
    if inds.is_empty() {
        return;
    }
    let mesh = Mesh {
        vertices: std::mem::take(verts),
        indices: std::mem::take(inds),
        texture: None, // texture blanche par défaut -> couleur du sommet
    };
    draw_mesh(&mesh);
    let mut v = mesh.vertices;
    v.clear();
    let mut i = mesh.indices;
    i.clear();
    *verts = v;
    *inds = i;
}
