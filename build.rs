use std::fs;
use std::process::Command;
use std::env;
use std::path::PathBuf;


fn main() {
    println!("cargo:rerun-if-changed=src/reg_write.s");
    assemble("src/targets/reg_write.s", "reg_write");
    println!("cargo:rerun-if-changed=src/reg_read.s");
    assemble("src/targets/reg_read.s", "reg_read");
}

fn assemble(infile: &str, outfile: &str) {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let exe_file = out_dir.join(outfile);
    let path_file = outfile.to_owned() + "_path";

    // Assemble
    let status = Command::new("gcc")
        .arg(infile)
        .arg("-o")
        .arg(&exe_file)
        .status()
        .expect("Failed to run assembler");
    assert!(status.success(), "Assembly failed");

    // Write the path for tests to find
    let _ = fs::write(
        out_dir.join(path_file),
        exe_file.display().to_string(),
    );

    // println!("cargo:reg-write={}", exe_file.display());
}
