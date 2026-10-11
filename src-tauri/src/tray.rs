use tauri::image::Image;
use tiny_skia::{LineCap, Paint, PathBuilder, Pixmap, Stroke, Transform};

type Rgb = (u8, u8, u8);

const SIZE: u32 = 32;
const STROKE: f32 = 2.4;
const CENTER: f32 = 16.0;
/// Geometry fills the 32px canvas so the 16px tray render stays readable.
const BAR: (f32, f32, f32, f32) = (16.0, 3.5, 16.0, 16.0);
const ARC_RADIUS: f32 = 11.5;
/// The power glyph: a bar plus a ~254-degree arc whose gap faces up.
const ARC_START: f32 = -36.87_f32.to_radians();
const ARC_END: f32 = 216.87_f32.to_radians();
const ARC_STEPS: usize = 64;

/// Renders the tray glyph with tiny-skia: near-black on a light taskbar,
/// white on a dark one. Geometry matches the approved Option B mockup.
pub fn render_glyph(light_taskbar: bool) -> Image<'static> {
    let foreground = foreground_color(light_taskbar);
    let mut pixmap = Pixmap::new(SIZE, SIZE).expect("tray icon dimensions are valid");

    let (bar_x, bar_top, _, bar_bottom) = BAR;
    stroke(&mut pixmap, foreground, &[(bar_x, bar_top), (bar_x, bar_bottom)]);
    stroke(&mut pixmap, foreground, &arc_points());

    Image::new_owned(pixmap.take_demultiplied(), SIZE, SIZE)
}

fn foreground_color(light_taskbar: bool) -> Rgb {
    if light_taskbar {
        (26, 26, 26)
    } else {
        (255, 255, 255)
    }
}

fn arc_points() -> Vec<(f32, f32)> {
    (0..=ARC_STEPS)
        .map(|step| {
            let angle = ARC_START + (ARC_END - ARC_START) * (step as f32 / ARC_STEPS as f32);
            (
                CENTER + ARC_RADIUS * angle.cos(),
                CENTER + ARC_RADIUS * angle.sin(),
            )
        })
        .collect()
}

fn stroke(pixmap: &mut Pixmap, color: Rgb, points: &[(f32, f32)]) {
    let mut builder = PathBuilder::new();
    let mut iter = points.iter();
    if let Some((x, y)) = iter.next() {
        builder.move_to(*x, *y);
    }
    for (x, y) in iter {
        builder.line_to(*x, *y);
    }
    let Some(path) = builder.finish() else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.0, color.1, color.2, 255);
    paint.anti_alias = true;
    pixmap.stroke_path(
        &path,
        &paint,
        &Stroke {
            width: STROKE,
            line_cap: LineCap::Round,
            ..Default::default()
        },
        Transform::identity(),
        None,
    );
}

/// Reads the taskbar theme; the tray icon lives on the taskbar, which follows
/// the system theme rather than the app theme.
pub fn taskbar_uses_light_theme() -> bool {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};

        let wide = |value: &str| value.encode_utf16().chain(std::iter::once(0)).collect::<Vec<_>>();
        let subkey = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
        let value_name = wide("SystemUsesLightTheme");
        let mut value = 0_u32;
        let mut size = u32::try_from(size_of_val(&value)).expect("DWORD size is valid");
        let result = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                subkey.as_ptr(),
                value_name.as_ptr(),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                std::ptr::from_mut(&mut value).cast(),
                &mut size,
            )
        };
        result == 0 && value == 1
    }
    #[cfg(not(target_os = "windows"))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn colored_pixels(image: &Image) -> usize {
        image
            .rgba()
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[3] > 40)
            .count()
    }

    fn strongest_pixel(image: &Image) -> [u8; 3] {
        image
            .rgba()
            .as_chunks::<4>()
            .0
            .iter()
            .max_by_key(|p| p[3])
            .map(|p| [p[0], p[1], p[2]])
            .expect("image has pixels")
    }

    #[test]
    fn renders_visible_glyph_on_both_themes() {
        for light in [true, false] {
            let image = render_glyph(light);
            assert_eq!(image.width(), SIZE);
            assert_eq!(image.height(), SIZE);
            assert!(colored_pixels(&image) > 100, "light={light}");
        }
    }

    #[test]
    fn foreground_matches_taskbar_theme() {
        let light = render_glyph(true);
        let dark = render_glyph(false);
        assert_ne!(strongest_pixel(&light), strongest_pixel(&dark));
        assert_eq!(strongest_pixel(&dark), [255, 255, 255]);
    }
}
