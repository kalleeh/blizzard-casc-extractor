use casc_extractor::casc::casclib_ffi::CascArchive;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let archive = CascArchive::open("/Applications/StarCraft")?;
    
    // Try different naming patterns for missing units
    let search_patterns = vec![
        // Terran
        ("Wraith", vec!["unit\\terran\\wraith.grp", "unit\\terran\\wraith1.grp", "unit\\terran\\wraith2.grp"]),
        ("Valkyrie", vec!["unit\\terran\\valkyrie.grp", "unit\\terran\\valk.grp", "unit\\terran\\valkyrie1.grp"]),
        ("Science Vessel", vec!["unit\\terran\\vessel.grp", "unit\\terran\\sciencevessel.grp", "unit\\terran\\scvessel.grp"]),
        // Protoss
        ("Dark Templar", vec!["unit\\protoss\\darktemplar.grp", "unit\\protoss\\darktemp.grp", "unit\\protoss\\darchon.grp"]),
        ("Dark Archon", vec!["unit\\protoss\\darkarch.grp", "unit\\protoss\\darchon.grp", "unit\\protoss\\darkarchon.grp"]),
        ("Reaver", vec!["unit\\protoss\\reaver.grp", "unit\\protoss\\reaver1.grp"]),
        ("Observer", vec!["unit\\protoss\\observer.grp", "unit\\protoss\\observ.grp"]),
        // Zerg
        ("Mutalisk", vec!["unit\\zerg\\mutalisk.grp", "unit\\zerg\\mutal.grp", "unit\\zerg\\muta.grp"]),
        ("Devourer", vec!["unit\\zerg\\devourer.grp", "unit\\zerg\\devour.grp", "unit\\zerg\\devourer1.grp"]),
        ("Scourge", vec!["unit\\zerg\\scourge.grp", "unit\\zerg\\scourg.grp"]),
    ];
    
    println!("🔍 Searching for missing units...\n");
    
    for (name, patterns) in search_patterns {
        print!("{}: ", name);
        let mut found = false;
        
        for pattern in patterns {
            match archive.extract_file(pattern) {
                Ok(data) => {
                    println!("✅ FOUND at \"{}\" ({} bytes)", pattern, data.len());
                    found = true;
                    break;
                }
                Err(_) => {}
            }
        }
        
        if !found {
            println!("❌ Not found in any pattern");
        }
    }
    
    Ok(())
}
