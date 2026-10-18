use std::fs;

use wesl::Compiler;

#[path = "src/shader_constants.rs"]
mod shader_constants;
use shader_constants::*;

const SHADER_DIR: &str = "src/shaders";
const CONSTANTS_PATH: &str = "src/shaders/lib/constants.wesl";

fn main() {
    println!("cargo:rerun-if-changed=src/shader_constants.rs");
    println!("cargo:rerun-if-changed={SHADER_DIR}");

    write_constants();

    let compiler = Compiler::default();
    for entry in fs::read_dir(SHADER_DIR).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|ext| ext != "wesl") {
            continue;
        }
        let name = path.file_stem().unwrap().to_str().unwrap();
        let module = format!("package::{name}").parse().unwrap();
        compiler
            .compile_module(SHADER_DIR, &module)
            .unwrap_or_else(|e| panic!("{name}: {e}"))
            .write_artifact(name);
    }
}

fn write_constants() {
    let constants = [
        ("bind_group_scene", format!("{BIND_GROUP_SCENE}u")),
        ("bind_group_model", format!("{BIND_GROUP_MODEL}u")),
        ("bind_group_transforms", format!("{BIND_GROUP_TRANSFORMS}u")),
        ("bind_group_material", format!("{BIND_GROUP_MATERIAL}u")),
        ("move_lights_workgroup_size", format!("{MOVE_LIGHTS_WORKGROUP_SIZE}u")),
        ("light_radius", format!("{LIGHT_RADIUS:?}f")),
    ];

    let src: String = constants
        .iter()
        .map(|(name, value)| format!("const {name} = {value};\n"))
        .collect();

    if fs::read_to_string(CONSTANTS_PATH).ok().as_deref() != Some(src.as_str()) {
        fs::write(CONSTANTS_PATH, src).unwrap();
    }
}
