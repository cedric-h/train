#![feature(array_map)]
use std::convert::TryInto;

use train::art::{ArtData, ArtIndicesBuilder, Track, Vertex, IMAGE_SIZE};

// Shamelessly lifted from `https://stackoverflow.com/a/42186553`.
unsafe fn as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    std::slice::from_raw_parts((p as *const T) as *const u8, std::mem::size_of::<T>())
}

fn main() {
    let (doc, datas, images) = gltf::import("train.glb").unwrap();
    let mut data = Box::new(ArtData::default());
    let mut art_indices_builder = ArtIndicesBuilder::default();
    let mut current_index = 0;
    let mut current_vert = 0;
    let mut overflow = false;

    for mesh in doc.meshes() {
        let start_index = current_index;
        let start_vert = current_vert;

        for prim in mesh.primitives() {
            let reader = prim.reader(|b| Some(&datas.get(b.index())?.0[..b.length()]));
            for ((pos, norm), uv) in reader
                .read_positions()
                .unwrap()
                .zip(reader.read_normals().unwrap())
                .zip(reader.read_tex_coords(0).unwrap().into_f32())
            {
                if let Some(vert) = data.vertices.get_mut(current_vert) {
                    *vert = Vertex { pos: pos.into(), norm: norm.into(), uv: uv.into() };
                } else {
                    overflow = true;
                }
                current_vert += 1;
            }

            for i in reader.read_indices().unwrap().into_u32() {
                if let Some(index) = data.indices.get_mut(current_index) {
                    *index = (start_vert as u32 + i).try_into().unwrap();
                } else {
                    overflow = true;
                }
                current_index += 1;
            }
        }

        let name = mesh.name().unwrap();
        println!("processing {}", name);
        art_indices_builder.insert(name, start_index, current_index - start_index);
    }

    println!(
        "verts space: {}, got: {} verts,\nindex space: {}, got: {} indices",
        data.vertices.len(),
        current_vert,
        data.indices.len(),
        current_index
    );
    if overflow {
        panic!("that's a geometry overflow :(");
    }

    let mut pixels = images.into_iter().next().unwrap().pixels.into_iter();
    data.image = [(); IMAGE_SIZE].map(|_| pixels.next().unwrap());

    data.art_indices = art_indices_builder.unwrap();

    let track: Vec<Vec<_>> =
        serde_json::from_str(&std::fs::read_to_string("track.json").unwrap()).unwrap();
    data.track = Track::from_points(track.iter().next().unwrap());

    data.last_occupied_vert = current_vert.try_into().unwrap();
    data.last_occupied_index = current_index.try_into().unwrap();

    unsafe {
        std::fs::write("train.cedset", as_u8_slice::<ArtData>(&data)).unwrap();
    }
    println!("done!");
}
