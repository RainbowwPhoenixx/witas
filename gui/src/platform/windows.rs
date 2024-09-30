use dll_syringe::{process::OwnedProcess, Syringe};

use super::get_library_path;

pub fn try_inject() {
    // find target process by name
    let Some(target_process) = OwnedProcess::find_first_by_name("witness64_d3d11") else {
        println!("Failed to find the witness process");
        return;
    };

    // create a new syringe for the target process
    let syringe = Syringe::for_process(target_process);

    let library = get_library_path();
    
    if let Err(e) = syringe.inject(library) {
        println!("Failed to inject dll: {e}")
    }
}
