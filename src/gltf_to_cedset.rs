#![feature(array_map)]
use std::convert::TryInto;

mod art;
use art::{Art, ArtData, Vertex, IMAGE_SIZE};

// Shamelessly lifted from `https://stackoverflow.com/a/42186553`.
unsafe fn as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    std::slice::from_raw_parts(
        (p as *const T) as *const u8,
        std::mem::size_of::<T>(),
    )
}

fn main() {
    let (doc, datas, images) = gltf::import("train.glb").unwrap();
    let mesh_data = doc.meshes().next().expect("no meshes");
    let mut data = Box::new(ArtData::default());
    let mut current_index = 0;
    let mut current_vert = 0;
    let mut train = None;
    let mut cart = None;
    let mut overflow = false;

    for mesh in doc.meshes() {
        let start_index = current_index;
        
        for prim in mesh_data.primitives() {
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
                    *index = i.try_into().unwrap();
                } else {
                    overflow = true;
                }
                current_index += 1;
            }
        }

        match mesh.name().unwrap() {
            "train" => train = Some((start_index, current_index)),
            "cart" => cart = Some((start_index, current_index)),
            other => panic!("{} is no mesh of ours!", other),
        }
    }

    if overflow {
        panic!(
            "expected verts: {}, got: {},\nexpected indices: {}, got: {}",
            data.vertices.len(),
            current_vert,
            data.indices.len(),
            current_index
        );
    }

    let mut pixels = images.into_iter().next().unwrap().pixels.into_iter();
    data.image = [(); IMAGE_SIZE].map(|_| pixels.next().unwrap());

    let art = Art {
        train: {
            let (start, end) = train.unwrap();
            (start.try_into().unwrap(), end.try_into().unwrap())
        },
        cart: {
            let (start, end) = cart.unwrap();
            (start.try_into().unwrap(), end.try_into().unwrap())
        },
    };

    let mut file = std::fs::File::create("train.cedset").unwrap();
    unsafe {
        use std::io::Write;
        file.write_all(as_u8_slice::<Art>(&art)).unwrap();
        file.write_all(as_u8_slice::<ArtData>(&data)).unwrap();
    }
}
