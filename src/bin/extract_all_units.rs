use casc_extractor::casc::casclib_ffi::CascArchive;
use casc_extractor::grp::GrpFile;
use std::path::Path;
use std::fs::File;
use std::io::BufWriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    println!("🎮 Comprehensive StarCraft Sprite Extraction");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let archive = CascArchive::open("/Applications/StarCraft")?;
    println!("✅ Opened CASC archive");
    
    let output_dir = Path::new("extracted/all_units");
    std::fs::create_dir_all(output_dir)?;
    
    // Comprehensive list of StarCraft unit sprites
    let sprite_files = vec![
        // Terran units
        ("Terran_Marine", "unit\\terran\\marine.grp"),
        ("Terran_Firebat", "unit\\terran\\firebat.grp"),
        ("Terran_Ghost", "unit\\terran\\ghost.grp"),
        ("Terran_Vulture", "unit\\terran\\vulture.grp"),
        ("Terran_Goliath", "unit\\terran\\goliath.grp"),
        ("Terran_SiegeTank_Tank", "unit\\terran\\tank.grp"),
        ("Terran_SiegeTank_Siege", "unit\\terran\\tsieged.grp"),
        ("Terran_SCV", "unit\\terran\\scv.grp"),
        ("Terran_Wraith", "unit\\terran\\wraith.grp"),
        ("Terran_Dropship", "unit\\terran\\dropship.grp"),
        ("Terran_Battlecruiser", "unit\\terran\\battlecr.grp"),
        ("Terran_Valkyrie", "unit\\terran\\valkyrie.grp"),
        ("Terran_ScienceVessel", "unit\\terran\\vessel.grp"),
        
        // Protoss units
        ("Protoss_Probe", "unit\\protoss\\probe.grp"),
        ("Protoss_Zealot", "unit\\protoss\\zealot.grp"),
        ("Protoss_Dragoon", "unit\\protoss\\dragoon.grp"),
        ("Protoss_HighTemplar", "unit\\protoss\\templar.grp"),
        ("Protoss_DarkTemplar", "unit\\protoss\\darktemplar.grp"),
        ("Protoss_Archon", "unit\\protoss\\archon.grp"),
        ("Protoss_DarkArchon", "unit\\protoss\\darkarch.grp"),
        ("Protoss_Shuttle", "unit\\protoss\\shuttle.grp"),
        ("Protoss_Scout", "unit\\protoss\\scout.grp"),
        ("Protoss_Corsair", "unit\\protoss\\corsair.grp"),
        ("Protoss_Carrier", "unit\\protoss\\carrier.grp"),
        ("Protoss_Arbiter", "unit\\protoss\\arbiter.grp"),
        ("Protoss_Reaver", "unit\\protoss\\reaver.grp"),
        
        // Zerg units
        ("Zerg_Larva", "unit\\zerg\\larva.grp"),
        ("Zerg_Drone", "unit\\zerg\\drone.grp"),
        ("Zerg_Zergling", "unit\\zerg\\zergling.grp"),
        ("Zerg_Hydralisk", "unit\\zerg\\hydra.grp"),
        ("Zerg_Ultralisk", "unit\\zerg\\ultra.grp"),
        ("Zerg_Mutalisk", "unit\\zerg\\mutalisk.grp"),
        ("Zerg_Guardian", "unit\\zerg\\guardian.grp"),
        ("Zerg_Devourer", "unit\\zerg\\devourer.grp"),
        ("Zerg_Scourge", "unit\\zerg\\scourge.grp"),
        ("Zerg_Queen", "unit\\zerg\\queen.grp"),
        ("Zerg_Defiler", "unit\\zerg\\defiler.grp"),
        ("Zerg_Overlord", "unit\\zerg\\overlord.grp"),
    ];
    
    let mut success_count = 0;
    let mut fail_count = 0;
    
    for (name, filename) in sprite_files {
        match archive.extract_file(filename) {
            Ok(data) => {
                match GrpFile::parse(&data) {
                    Ok(grp) => {
                        if let Some(frame) = grp.get_first_frame() {
                            if let Ok(rgba) = frame.to_rgba() {
                                let png_path = output_dir.join(format!("{}.png", name));
                                
                                let file = File::create(&png_path)?;
                                let w = BufWriter::new(file);
                                let mut encoder = png::Encoder::new(w, frame.width as u32, frame.height as u32);
                                encoder.set_color(png::ColorType::Rgba);
                                encoder.set_depth(png::BitDepth::Eight);
                                
                                let mut writer = encoder.write_header()?;
                                writer.write_image_data(&rgba)?;
                                
                                println!("✅ {}: {}x{}, {} frames", name, grp.width, grp.height, grp.frame_count);
                                success_count += 1;
                            }
                        }
                    }
                    Err(e) => {
                        println!("⚠️  {}: Parse error - {}", name, e);
                        fail_count += 1;
                    }
                }
            }
            Err(_) => {
                println!("❌ {}: File not found", name);
                fail_count += 1;
            }
        }
    }
    
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Success: {} | ❌ Failed: {}", success_count, fail_count);
    println!("📁 Saved to: {:?}", output_dir);
    
    Ok(())
}
