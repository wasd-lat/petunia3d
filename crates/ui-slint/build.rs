use std::{collections::HashMap, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=ui");

    let lucide_path = PathBuf::from(lucide_slint::lib().to_string());
    let libraries = HashMap::from([(String::from("lucide"), lucide_path)]);
    let config = slint_build::CompilerConfiguration::new().with_library_paths(libraries);

    slint_build::compile_with_config("ui/app.slint", config)
        .expect("failed to compile the experimental Slint frontend");
}
