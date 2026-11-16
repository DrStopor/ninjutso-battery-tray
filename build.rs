use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;

#[derive(Clone, Copy)]
struct Rgba {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Rgba {
    const fn transparent() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }
    }

    const fn opaque(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}

struct IconImage {
    width: u16,
    height: u16,
    bits: u16,
    data: Vec<u8>,
}

fn main() -> io::Result<()> {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return Ok(());
    }

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by cargo"));
    let ico_path = out_dir.join("app_icon.ico");
    let rc_path = out_dir.join("app_icon.rc");
    let res_path = out_dir.join("app_icon.res");
    let images = [16u16, 24, 32, 48, 64, 128, 256]
        .into_iter()
        .map(build_icon_image)
        .collect::<Vec<_>>();

    fs::write(&ico_path, build_ico_file(&images))?;
    fs::write(&rc_path, resource_script(&ico_path))?;
    compile_resource(&rc_path, &res_path)?;

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=assets/windows/app.manifest.xml");
    println!("cargo:rerun-if-env-changed=RC");
    println!(
        "cargo:rustc-link-arg-bin=ninjutso-battery-tray={}",
        res_path.display()
    );

    Ok(())
}

fn resource_script(ico_path: &std::path::Path) -> String {
    let version = env::var("CARGO_PKG_VERSION").expect("Cargo provides the package version");
    let numeric_version = [
        env::var("CARGO_PKG_VERSION_MAJOR").unwrap(),
        env::var("CARGO_PKG_VERSION_MINOR").unwrap(),
        env::var("CARGO_PKG_VERSION_PATCH").unwrap(),
        "0".to_string(),
    ]
    .join(",");
    let icon = ico_path.display().to_string().replace('\\', "\\\\");
    let manifest = if env::var_os("CARGO_FEATURE_GPUI_UI").is_some() {
        let path = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap())
            .join("assets/windows/app.manifest.xml");
        format!(
            "1 24 \"{}\"\n",
            path.display().to_string().replace('\\', "\\\\")
        )
    } else {
        String::new()
    };
    format!(
        r#"{manifest}1 ICON "{icon}"
1 VERSIONINFO
FILEVERSION {numeric_version}
PRODUCTVERSION {numeric_version}
FILEFLAGSMASK 0x3fL
FILEFLAGS 0x0L
FILEOS 0x40004L
FILETYPE 0x1L
FILESUBTYPE 0x0L
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904b0"
    BEGIN
      VALUE "CompanyName", "drstopor\0"
      VALUE "FileDescription", "Ninjutso Battery Tray\0"
      VALUE "FileVersion", "{version}\0"
      VALUE "InternalName", "ninjutso-battery-tray\0"
      VALUE "OriginalFilename", "ninjutso-battery-tray.exe\0"
      VALUE "ProductName", "Ninjutso Battery Tray\0"
      VALUE "ProductVersion", "{version}\0"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x0409, 1200
  END
END
"#
    )
}

fn compile_resource(rc_path: &PathBuf, res_path: &PathBuf) -> io::Result<()> {
    let rc_exe = find_rc_exe().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "rc.exe was not found. Install Windows SDK or set RC to rc.exe path.",
        )
    })?;

    let status = Command::new(&rc_exe)
        .arg("/nologo")
        .arg("/fo")
        .arg(res_path)
        .arg(rc_path)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{} failed with status {status}",
            rc_exe.display()
        )))
    }
}

fn find_rc_exe() -> Option<PathBuf> {
    if let Some(path) = env::var_os("RC")
        .map(PathBuf::from)
        .filter(|path| path.exists())
    {
        return Some(path);
    }

    let mut roots = Vec::new();
    if let Some(program_files_x86) = env::var_os("ProgramFiles(x86)") {
        roots.push(
            PathBuf::from(program_files_x86)
                .join("Windows Kits")
                .join("10")
                .join("bin"),
        );
    }
    roots.push(PathBuf::from(r"C:\Program Files (x86)\Windows Kits\10\bin"));

    let mut candidates = Vec::new();
    for root in roots {
        if let Ok(versions) = fs::read_dir(root) {
            for version in versions.flatten() {
                let path = version.path().join("x64").join("rc.exe");
                if path.exists() {
                    candidates.push(path);
                }
            }
        }
    }

    candidates.sort();
    candidates.pop()
}

fn build_icon_image(size: u16) -> IconImage {
    let size_i = size as i32;
    let mut pixels = vec![Rgba::transparent(); size as usize * size as usize];
    let scale = size as f32 / 256.0;

    draw_circle(
        &mut pixels,
        size_i,
        128.0,
        128.0,
        116.0,
        Rgba::opaque(18, 22, 28),
        scale,
    );
    draw_circle(
        &mut pixels,
        size_i,
        128.0,
        128.0,
        104.0,
        Rgba::opaque(35, 42, 50),
        scale,
    );
    draw_battery(&mut pixels, size_i, scale);
    draw_mouse(&mut pixels, size_i, scale);

    IconImage {
        width: size,
        height: size,
        bits: 32,
        data: encode_icon_dib(size, &pixels),
    }
}

fn draw_battery(pixels: &mut [Rgba], size: i32, scale: f32) {
    let black = Rgba::opaque(7, 10, 13);
    let white = Rgba::opaque(238, 244, 248);
    let green = Rgba::opaque(38, 190, 92);
    let dark = Rgba::opaque(22, 27, 32);

    fill_rect_scaled(pixels, size, 38.0, 88.0, 180.0, 82.0, black, scale);
    fill_rect_scaled(pixels, size, 46.0, 96.0, 164.0, 66.0, white, scale);
    fill_rect_scaled(pixels, size, 58.0, 108.0, 140.0, 42.0, dark, scale);
    fill_rect_scaled(pixels, size, 58.0, 108.0, 106.0, 42.0, green, scale);
    fill_rect_scaled(pixels, size, 218.0, 111.0, 16.0, 36.0, black, scale);
    fill_rect_scaled(pixels, size, 218.0, 120.0, 10.0, 18.0, white, scale);
}

fn draw_mouse(pixels: &mut [Rgba], size: i32, scale: f32) {
    let shadow = Rgba::opaque(3, 5, 7);
    let body = Rgba::opaque(244, 248, 250);
    let cut = Rgba::opaque(36, 44, 52);

    draw_ellipse(
        pixels, size, 128.0, 126.0, 58.0, 84.0, shadow, scale, 4.0, 6.0,
    );
    draw_ellipse(
        pixels, size, 128.0, 122.0, 54.0, 80.0, body, scale, 0.0, 0.0,
    );

    fill_rect_scaled(pixels, size, 124.0, 48.0, 9.0, 38.0, cut, scale);
    fill_rect_scaled(pixels, size, 91.0, 87.0, 74.0, 9.0, cut, scale);
    fill_rect_scaled(pixels, size, 124.0, 96.0, 9.0, 58.0, cut, scale);
    fill_rect_scaled(pixels, size, 119.0, 61.0, 19.0, 31.0, body, scale);
}

fn draw_circle(
    pixels: &mut [Rgba],
    size: i32,
    cx: f32,
    cy: f32,
    radius: f32,
    color: Rgba,
    scale: f32,
) {
    draw_ellipse(pixels, size, cx, cy, radius, radius, color, scale, 0.0, 0.0);
}

#[allow(clippy::too_many_arguments)]
fn draw_ellipse(
    pixels: &mut [Rgba],
    size: i32,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    color: Rgba,
    scale: f32,
    offset_x: f32,
    offset_y: f32,
) {
    let cx = (cx + offset_x) * scale;
    let cy = (cy + offset_y) * scale;
    let rx = (rx * scale).max(1.0);
    let ry = (ry * scale).max(1.0);

    for y in 0..size {
        for x in 0..size {
            let dx = (x as f32 + 0.5 - cx) / rx;
            let dy = (y as f32 + 0.5 - cy) / ry;
            if dx * dx + dy * dy <= 1.0 {
                set_pixel(pixels, size, x, y, color);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn fill_rect_scaled(
    pixels: &mut [Rgba],
    size: i32,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: Rgba,
    scale: f32,
) {
    let left = (x * scale).round() as i32;
    let top = (y * scale).round() as i32;
    let right = ((x + width) * scale).round() as i32;
    let bottom = ((y + height) * scale).round() as i32;

    for yy in top..bottom {
        for xx in left..right {
            set_pixel(pixels, size, xx, yy, color);
        }
    }
}

fn set_pixel(pixels: &mut [Rgba], size: i32, x: i32, y: i32, color: Rgba) {
    if x >= 0 && x < size && y >= 0 && y < size {
        pixels[(y * size + x) as usize] = color;
    }
}

fn encode_icon_dib(size: u16, pixels: &[Rgba]) -> Vec<u8> {
    let width = size as u32;
    let height = size as u32;
    let mask_stride = width.div_ceil(32) * 4;
    let image_size = width * height * 4;
    let mask_size = mask_stride * height;
    let mut data = Vec::with_capacity(40 + image_size as usize + mask_size as usize);

    push_u32(&mut data, 40);
    push_i32(&mut data, width as i32);
    push_i32(&mut data, (height * 2) as i32);
    push_u16(&mut data, 1);
    push_u16(&mut data, 32);
    push_u32(&mut data, 0);
    push_u32(&mut data, image_size);
    push_i32(&mut data, 0);
    push_i32(&mut data, 0);
    push_u32(&mut data, 0);
    push_u32(&mut data, 0);

    for y in (0..height).rev() {
        for x in 0..width {
            let p = pixels[(y * width + x) as usize];
            data.push(p.b);
            data.push(p.g);
            data.push(p.r);
            data.push(p.a);
        }
    }

    data.resize(data.len() + mask_size as usize, 0);
    data
}

fn build_ico_file(images: &[IconImage]) -> Vec<u8> {
    let mut data = Vec::new();
    push_u16(&mut data, 0);
    push_u16(&mut data, 1);
    push_u16(&mut data, images.len() as u16);

    let mut offset = 6 + images.len() * 16;
    for image in images {
        data.push(if image.width >= 256 {
            0
        } else {
            image.width as u8
        });
        data.push(if image.height >= 256 {
            0
        } else {
            image.height as u8
        });
        data.push(0);
        data.push(0);
        push_u16(&mut data, 1);
        push_u16(&mut data, image.bits);
        push_u32(&mut data, image.data.len() as u32);
        push_u32(&mut data, offset as u32);
        offset += image.data.len();
    }

    for image in images {
        data.extend_from_slice(&image.data);
    }

    data
}

fn push_u16(data: &mut Vec<u8>, value: u16) {
    data.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(data: &mut Vec<u8>, value: u32) {
    data.extend_from_slice(&value.to_le_bytes());
}

fn push_i32(data: &mut Vec<u8>, value: i32) {
    data.extend_from_slice(&value.to_le_bytes());
}
