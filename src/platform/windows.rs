use dll_syringe::{process::OwnedProcess, Syringe};

pub fn try_inject() {
    // find target process by name
    let Some(target_process) = OwnedProcess::find_first_by_name("witness64_d3d11") else {
        println!("Failed to find the witness process");
        return;
    };

    // create a new syringe for the target process
    let syringe = Syringe::for_process(target_process);
    
    if let Err(e) = syringe.inject("./witness_tas.dll") {
        println!("Failed to inject dll: {e}")
    }

    // if let Err(e) = syringe.inject("./target/release/witness_tas.dll") {
    //     println!("Failed to inject dll: {e}")
    // }
}
