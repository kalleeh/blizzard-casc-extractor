use casc_extractor::casc::casclib_ffi::CascArchive;
use casc_extractor::grp::GrpFile;
use std::path::Path;
use std::fs::File;
use std::io::BufWriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let archive = CascArchive::open("/Applications/StarCraft")?;
    let output_dir = Path::new("extracted/all_sprite_sheets");
    std::fs::create_dir_all(output_dir)?;
    
    let units = vec![
        // Terran
        ("Terran_Marine", "unit\\terran\\marine.grp"),
        ("Terran_Firebat", "unit\\terran\\firebat.grp"),
        ("Terran_Ghost", "unit\\terran\\ghost.grp"),
        ("Terran_Vulture", "unit\\terran\\vulture.grp"),
        ("Terran_Goliath", "unit\\terran\\goliath.grp"),
        ("Terran_SiegeTank", "unit\\terran\\tank.grp"),
        ("Terran_SCV", "unit\\terran\\scv.grp"),
        ("Terran_Wraith", "unit\\terran\\wraith.grp"),
        ("Terran_Dropship", "unit\\terran\\dropship.grp"),
        ("Terran_Battlecruiser", "unit\\terran\\battlecr.grp"),
        ("Terran_Valkyrie", "unit\\terran\\valkyrie.grp"),
        ("Terran_ScienceVessel", "unit\\terran\\vessel.grp"),
        // Protoss
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
        ("Protoss_Observer", "unit\\protoss\\observer.grp"),
        // Zerg
        ("Zerg_Larva", "unit\\zerg\\larva.grp"),
        ("Zerg_Egg", "unit\\zerg\\egg.grp"),
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
    
    let mut success = 0;
    let mut failed = 0;
    
    for (name, path) in units {
        match archive.extract_file(path) {
            Ok(data) => {
                match GrpFile::parse(&data) {
                    Ok(grp) => {
                        let frames_per_row = 17;
                        let rows = (grp.frame_count as usize + frames_per_row - 1) / frames_per_row;
                        let sheet_width = grp.width as u32 * frames_per_row as u32;
                        let sheet_height = grp.height as u32 * rows as u32;
                        
                        let mut sheet_data = vec![0u8; (sheet_width * sheet_height * 4) as usize];
                        
                        for (idx, frame) in grp.frames.iter().enumerate() {
                            if let Ok(rgba) = frame.to_rgba() {
                                let row = idx / frames_per_row;
                                let col = idx % frames_per_row;
                                let x_offset = col * grp.width as usize;
                                let y_offset = row * grp.height as usize;
                                
                                for y in 0..frame.height as usize {
                                    for x in 0..frame.width as usize {
                                        let src_idx = (y * frame.width as usize + x) * 4;
                                        let dst_x = x_offset + x;
                                        let dst_y = y_offset + y;
                                        let dst_idx = (dst_y * sheet_width as usize + dst_x) * 4;
                                        sheet_data[dst_idx..dst_idx + 4].copy_from_slice(&rgba[src_idx..src_idx + 4]);
                                    }
                                }
                            }
                        }
                        
                        let png_path = output_dir.join(format!("{}.png", name));
                        let file = File::create(&png_path)?;
                        let w = BufWriter::new(file);
                        let mut encoder = png::Encoder::new(w, sheet_width, sheet_height);
                        encoder.set_color(png::ColorType::Rgba);
                        encoder.set_depth(png::BitDepth::Eight);
                        let mut writer = encoder.write_header()?;
                        writer.write_image_data(&sheet_data)?;
                        
                        println!("✅ {}: {}x{} ({} frames)", name, sheet_width, sheet_height, grp.frame_count);
                        success += 1;
                    }
                    Err(e) => {
                        println!("⚠️  {}: Parse error - {}", name, e);
                        failed += 1;
                    }
                }
            }
            Err(_) => {
                println!("❌ {}: Not found", name);
                failed += 1;
            }
        }
    }
    
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Success: {} | ❌ Failed: {}", success, failed);
    println!("📁 Saved to: {:?}", output_dir);
    
    Ok(())
}
