use substrait_expr_funcgen::{generate_functions, Options};

fn main() {
    println!("cargo:rerun-if-changed=../substrait/extensions");
    println!("cargo:rerun-if-changed=build.rs");

    let options = Options {
        crate_name: Some("crate".to_string()),
        ..Default::default()
    };

    generate_functions(&[(
        "https://github.com/substrait-io/substrait/blob/main/extensions/functions_arithmetic.yaml",
        "../substrait/extensions/functions_arithmetic.yaml",
    ), (
        "https://github.com/substrait-io/substrait/blob/main/extensions/functions_boolean.yaml",
        "../substrait/extensions/functions_boolean.yaml",
    ), (
        "https://github.com/substrait-io/substrait/blob/main/extensions/functions_comparison.yaml",
        "../substrait/extensions/functions_comparison.yaml",
    ), (
        "https://github.com/substrait-io/substrait/blob/main/extensions/functions_datetime.yaml",
        "../substrait/extensions/functions_datetime.yaml",
    )], options)
    .unwrap();
}
