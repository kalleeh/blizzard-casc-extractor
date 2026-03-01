use casc_extractor::casc::casclib_ffi::CascArchive;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let archive = CascArchive::open("/Applications/StarCraft/Data/data")?;
    
    println!("Searching for ANIM files...\n");
    
    // Check main_XXX.anim pattern
    for i in 0..200 {
        let path = format!("anim/main_{:03}.anim", i);
        if archive.file_exists(&path) {
            if let Ok(data) = archive.read_file(&path) {
                println!("{}: {} bytes", path, data.len());
            }
        }
    }
    
    Ok(())
}
