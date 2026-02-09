mod triangle;

use clap::Parser;
use nalgebra::Vector3;
use ply_rs_bw::{
    parser,
    ply::{self, Addable},
    writer::Writer,
};
use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::PathBuf,
};
use triangle::Mesh;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    sampling_distance: f64,
    input_path: PathBuf,
    output_path: PathBuf,
}

fn main() {
    let args = Args::parse();

    let (vertices, faces) = load_mesh(&args.input_path);

    let mut mesh = Mesh::new(vertices, faces);

    mesh.split_edges_until(args.sampling_distance);

    save_mesh(&args.output_path, args.sampling_distance, &mesh);
}

#[allow(clippy::type_complexity)]
fn load_mesh(input_path: &PathBuf) -> (Vec<Vector3<f64>>, Vec<(usize, usize, usize)>) {
    let f = File::open(input_path).unwrap();

    let mut f = BufReader::new(f);

    let parser = parser::Parser::<ply::DefaultElement>::new();

    let ply = parser.read_ply(&mut f).unwrap();

    let vertices: Vec<_> = ply.payload["vertex"]
        .iter()
        .map(|vertex| {
            let x = property_to_float(&vertex["x"]);
            let y = property_to_float(&vertex["y"]);
            let z = property_to_float(&vertex["z"]);

            Vector3::new(x, y, z)
        })
        .collect();

    let faces: Vec<_> = ply.payload["face"]
        .iter()
        .map(|face| {
            let list = property_to_usize_vec(&face["vertex_indices"]);

            (list[0], list[1], list[2])
        })
        .collect();
    (vertices, faces)
}

fn save_mesh(output_path: &PathBuf, sampling_distance: f64, mesh: &Mesh) {
    let mut ply = ply::Ply::<ply::DefaultElement>::new();
    ply.header.encoding = ply::Encoding::BinaryLittleEndian;
    ply.header.encoding = ply::Encoding::Ascii;

    let mut sampling_distance_element = ply::ElementDef::new("sampling_distance".to_string());

    let p = ply::PropertyDef::new(
        "sampling_distance".to_string(),
        ply::PropertyType::Scalar(ply::ScalarType::Double),
    );
    sampling_distance_element.properties.add(p);
    ply.header.elements.add(sampling_distance_element);

    let mut vertex_element = ply::ElementDef::new("vertex".to_string());
    let p = ply::PropertyDef::new(
        "x".to_string(),
        ply::PropertyType::Scalar(ply::ScalarType::Float),
    );
    vertex_element.properties.add(p);
    let p = ply::PropertyDef::new(
        "y".to_string(),
        ply::PropertyType::Scalar(ply::ScalarType::Float),
    );
    vertex_element.properties.add(p);
    let p = ply::PropertyDef::new(
        "z".to_string(),
        ply::PropertyType::Scalar(ply::ScalarType::Float),
    );
    vertex_element.properties.add(p);
    ply.header.elements.add(vertex_element);

    let mut face_element = ply::ElementDef::new("face".to_string());
    let p = ply::PropertyDef::new(
        "vertex_indices".to_string(),
        ply::PropertyType::List(ply::ScalarType::UChar, ply::ScalarType::Int),
    );
    face_element.properties.add(p);
    ply.header.elements.add(face_element);

    ply.payload.insert(
        "sampling_distance".to_string(),
        vec![{
            let mut s = ply::DefaultElement::new();
            s.insert(
                "sampling_distance".to_string(),
                ply::Property::Double(sampling_distance),
            );
            s
        }],
    );

    let vertex: Vec<_> = mesh
        .vertices
        .iter()
        .map(|vertex| {
            let mut v = ply::DefaultElement::new();

            v.insert("x".to_string(), ply::Property::Float(vertex.x as f32));
            v.insert("y".to_string(), ply::Property::Float(vertex.y as f32));
            v.insert("z".to_string(), ply::Property::Float(vertex.z as f32));

            v
        })
        .collect();
    ply.payload.insert("vertex".to_string(), vertex);

    let face: Vec<_> = mesh
        .faces
        .iter()
        .map(|face| {
            let mut f = ply::DefaultElement::new();

            f.insert(
                "vertex_indices".to_string(),
                ply::Property::ListInt(face.vertex_indices.iter().map(|i| *i as i32).collect()),
            );

            f
        })
        .collect();
    ply.payload.insert("face".to_string(), face);

    ply.make_consistent().unwrap();

    let f = File::create(output_path).unwrap();
    let mut f = BufWriter::new(f);
    let w = Writer::new();
    w.write_ply(&mut f, &mut ply).unwrap();
}

fn property_to_float(prop: &ply::Property) -> f64 {
    use ply::Property;

    match prop {
        Property::Float(f) => *f as f64,
        Property::Double(f) => *f,
        _ => panic!(),
    }
}

fn property_to_usize_vec(prop: &ply::Property) -> Vec<usize> {
    use ply::Property;

    match prop {
        Property::ListUChar(f) => f.iter().map(|e| *e as usize).collect(),
        Property::ListUShort(f) => f.iter().map(|e| *e as usize).collect(),
        Property::ListUInt(f) => f.iter().map(|e| *e as usize).collect(),
        Property::ListInt(f) => f.iter().map(|e| *e as usize).collect(),
        _ => panic!(),
    }
}
