/// Font configuration and loading.

use eframe::egui;
use std::sync::Arc;

/// Font data
const JETBRAINS_MONO_BYTES: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
const SYMBOLS_NERD_BYTES: &[u8] = include_bytes!("../assets/fonts/SymbolsNerdFont-Regular.ttf");
const NOTO_SYMBOLS_BYTES: &[u8] = include_bytes!("../assets/fonts/NotoSansSymbols-Regular.ttf");
const NOTO_SYMBOLS2_BYTES: &[u8] = include_bytes!("../assets/fonts/NotoSansSymbols2-Regular.ttf");
const NOTO_EMOJI_BYTES: &[u8] = include_bytes!("../assets/fonts/NotoEmoji-Regular.ttf");

/// Configure fonts: JetBrains Mono primary, Nerd Font + Noto Emoji fallbacks.
pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Add font data
    fonts.font_data.insert(
        "jetbrains_mono".to_owned(),
        Arc::new(egui::FontData::from_static(JETBRAINS_MONO_BYTES)),
    );
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

    // Monospace: JetBrains Mono first, then Nerd Font, then Symbols, then Emoji
    let mono = fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap();
    mono.insert(0, "jetbrains_mono".to_owned());
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
