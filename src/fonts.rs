/// Font configuration and loading.

use eframe::egui;
use std::sync::Arc;

/// Font data
const JETBRAINS_MONO_BYTES: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
const SYMBOLS_NERD_BYTES: &[u8] = include_bytes!("../assets/fonts/SymbolsNerdFont-Regular.ttf");
const NOTO_SYMBOLS_BYTES: &[u8] = include_bytes!("../assets/fonts/NotoSansSymbols-Regular.ttf");
const NOTO_SYMBOLS2_BYTES: &[u8] = include_bytes!("../assets/fonts/NotoSansSymbols2-Regular.ttf");
const NOTO_EMOJI_BYTES: &[u8] = include_bytes!("../assets/fonts/NotoEmoji-Regular.ttf");

/// Try to load a system font by family name using font-kit.
/// Returns the font data bytes on success.
fn load_system_font(family: &str) -> Option<Vec<u8>> {
    use font_kit::family_name::FamilyName;
    use font_kit::properties::Properties;
    use font_kit::source::SystemSource;

    let source = SystemSource::new();
    match source.select_best_match(&[FamilyName::Title(family.to_string())], &Properties::new()) {
        Ok(handle) => match handle.load() {
            Ok(font) => match font.copy_font_data() {
                Some(data) => {
                    log::info!("Loaded system font: {}", family);
                    Some((*data).clone())
                }
                None => {
                    log::warn!("System font '{}' found but could not read font data", family);
                    None
                }
            },
            Err(e) => {
                log::warn!("Failed to load system font '{}': {}", family, e);
                None
            }
        },
        Err(e) => {
            log::warn!("System font '{}' not found: {}", family, e);
            None
        }
    }
}

/// Configure fonts: primary monospace font + Nerd Font + Noto Emoji fallbacks.
///
/// If `font_family` is `Some`, attempts to load that system font as the primary
/// monospace font. Falls back to embedded JetBrains Mono on failure.
pub fn setup_fonts(ctx: &egui::Context, font_family: Option<&str>) {
    let mut fonts = egui::FontDefinitions::default();

    // Determine primary monospace font
    let (primary_name, primary_data) = if let Some(family) = font_family {
        if let Some(data) = load_system_font(family) {
            ("custom_mono".to_owned(), Arc::new(egui::FontData::from_owned(data)))
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
