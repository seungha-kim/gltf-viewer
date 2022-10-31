use std::env;
use gltf::*;

fn main() {
    for (i, arg) in env::args().enumerate() {
        println!("arg {}: {}", i, arg);
    }
    let args = env::args().collect::<Vec<_>>();
    let file = Gltf::open(&args[1]).unwrap();

    println!("extension used:");
    for ext in file.extensions_used() {
        println!("- {}", ext);
    }

    println!("extension required:");
    for ext in file.extensions_required() {
        println!("= {}", ext);
    }

    for view in file.views() {
        println!("view with index {}, target {:?}, length {}, stride {:?}", view.index(), view.target(), view.length(), view.stride());
    }

    for mat in file.materials() {
        println!("material {:?}, pbr color {:?}", mat.index(), mat.pbr_metallic_roughness().base_color_factor());
    }

    for scene in file.scenes() {
        println!("scene: {}", scene.name().unwrap_or("No name"));
        for node in scene.nodes() {
            print_node_hierarchy(node, 0);
        }
    }
    println!("Hello, world!");
}

fn print_node_hierarchy(node: Node, level: usize) {
    let indent = "  ".repeat(level);
    println!("{}node: {}", indent, node.name().unwrap_or("No name"));
    if let Some(mesh) = node.mesh() {
        println!("{}- has mesh with {} primitives", indent, mesh.primitives().count());
        for (i, primitive) in mesh.primitives().enumerate() {
            println!("{}- primitive {}", indent, i);
            for (semantic, accessor) in primitive.attributes() {
                println!("{}- semantic {:?}, accessor view index {}, offset {}", indent, semantic, accessor.view().unwrap().index(), accessor.offset());
                // 필요한 view 를 기록해두고, 나중에 view 업로드 한다.
                // 근데 blender 는 그냥 accessor 마다 전부 개별로 view 를 만들어버렸네
            }
        }
    }

    for child in node.children() {
        print_node_hierarchy(child, level + 1);
    }
}
