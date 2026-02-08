/// Font configuration and loading.

use eframe::egui;
use std::sync::Arc;

/// Font data
const JETBRAINS_MONO_BYTES: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
const SYMBOLS_NERD_BYTES: &[u8] = include_bytes!("../assets/fonts/SymbolsNerdFont-Regular.ttf");
const NOTO_SYMBOLS_BYTES: &[u8] = include_bytes!("../assets/fonts/NotoSansSymbols-Regular.ttf");
const NOTO_SYMBOLS2_BYTES: &[u8] = include_bytes!("../assets/fonts/NotoSansSymbols2-Regular.ttf");
const NOTO_EMOJI_BYTES: &[u8] = include_bytes!("../assets/fonts/NotoEmoji-Regular.ttf");

/// Given font file bytes and a PostScript name, find the matching face index.
/// For TTC (collection) files, iterates faces to match by PostScript name.
/// For single font files, returns 0.
fn find_font_index(data: &[u8], postscript_name: &str) -> u32 {
    if let Some(n) = ttf_parser::fonts_in_collection(data) {
        for i in 0..n {
            if let Ok(face) = ttf_parser::Face::parse(data, i) {
                for name in face.names() {
                    if name.name_id == ttf_parser::name_id::POST_SCRIPT_NAME {
                        if let Some(ps_name) = name.to_string() {
                            if ps_name == postscript_name {
                                return i;
                            }
                        }
                    }
                }
            }
        }
        log::warn!(
            "PostScript name '{}' not found in TTC ({} faces), using index 0",
            postscript_name, n
        );
    }
    0
}

/// Look up a system font by family name using Core Text (CTFontCreateWithName),
/// then read the font file from disk. Returns (font_bytes, font_index).
///
/// This uses the same fast code path as Alacritty and other terminals —
/// a direct name lookup, not the expensive font descriptor matching that font-kit uses.
fn load_system_font(family: &str) -> Option<(Vec<u8>, u32)> {
    // CTFontCreateWithName — fast direct lookup
    let ct_font = core_text::font::new_from_name(family, 16.0).ok()?;

    // Get font file path
    let path = ct_font.url()?.to_path()?;
    log::info!("Core Text resolved '{}' to {}", family, path.display());

    // Read font file
    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(e) => {
            log::warn!("Failed to read font file {}: {}", path.display(), e);
            return None;
        }
    };

    // Find correct index within TTC files
    let postscript_name = ct_font.postscript_name();
    let font_index = find_font_index(&data, &postscript_name);
    log::info!(
        "Loaded '{}' (PostScript: {}, index: {}, {} bytes)",
        family, postscript_name, font_index, data.len()
    );

    Some((data, font_index))
}

/// Configure fonts: primary monospace font + Nerd Font + Noto Emoji fallbacks.
///
/// If `font_family` is `Some`, attempts to load that system font as the primary
/// monospace font. Falls back to embedded JetBrains Mono on failure.
pub fn setup_fonts(ctx: &egui::Context, font_family: Option<&str>) {
    let mut fonts = egui::FontDefinitions::default();

    // Determine primary monospace font
    let (primary_name, primary_data) = if let Some(family) = font_family {
        if let Some((data, font_index)) = load_system_font(family) {
            let mut font_data = egui::FontData::from_owned(data);
            font_data.index = font_index;
            ("custom_mono".to_owned(), Arc::new(font_data))
        } else {
            log::warn!("Falling back to embedded JetBrains Mono");
            ("jetbrains_mono".to_owned(), Arc::new(egui::FontData::from_static(JETBRAINS_MONO_BYTES)))
        }
    } else {
        ("jetbrains_mono".to_owned(), Arc::new(egui::FontData::from_static(JETBRAINS_MONO_BYTES)))
    };

    // Add font data
    fonts.font_data.insert(primary_name.clone(), primary_data);
    fonts.font_data.insert(
        "symbols_nerd".to_owned(),
        Arc::new(egui::FontData::from_static(SYMBOLS_NERD_BYTES)),
    );
    fonts.font_data.insert(
        "noto_symbols".to_owned(),
        Arc::new(egui::FontData::from_static(NOTO_SYMBOLS_BYTES)),
    );
    fonts.font_data.insert(
        "noto_symbols2".to_owned(),
        Arc::new(egui::FontData::from_static(NOTO_SYMBOLS2_BYTES)),
    );
    fonts.font_data.insert(
        "noto_emoji".to_owned(),
        Arc::new(egui::FontData::from_static(NOTO_EMOJI_BYTES)),
    );

    // Monospace: primary font first, then Nerd Font, then Symbols, then Emoji
    let mono = fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap();
    mono.insert(0, primary_name.clone());
    mono.push("symbols_nerd".to_owned());
    mono.push("noto_symbols".to_owned());
    mono.push("noto_symbols2".to_owned());
    mono.push("noto_emoji".to_owned());

    // Proportional: keep defaults but add fallbacks
    let prop = fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap();
    prop.push("symbols_nerd".to_owned());
    prop.push("noto_symbols".to_owned());
    prop.push("noto_symbols2".to_owned());
    prop.push("noto_emoji".to_owned());

    ctx.set_fonts(fonts);

    // Install image loaders for PNG support
    egui_extras::install_image_loaders(ctx);
}
