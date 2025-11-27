use std::io::Write;
use serde_json::Value;
use image::GenericImage;
use image::{DynamicImage, ImageBuffer, RgbaImage};

type CoordsTuple = (u32, u32, u32, u32, u32, u32);
#[derive(Debug)]
struct Views {
    width: u32,
    height: u32,
    coords: Vec<CoordsTuple>,
}

impl std::fmt::Display for Views {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Views {{ width: {}, height: {}, coords: {:?} }}", self.width, self.height, self.coords)
    }
}

fn parse_json(json_str: &str) -> Result<Views, Box<dyn std::error::Error>> {
    let value: Value = serde_json::from_str(&json_str)?;
    
    // Get the views
    let views_json = value["views"]
        .get(0)
        .ok_or("No views field in the json")?
        .as_object()
        .ok_or("No views field in the json")?;
    let width = views_json["width"]
        .as_u64()
        .ok_or("Failed to parse width from json")?
        as u32;
    let height = views_json["height"]
        .as_u64()
        .ok_or("Failed to parse height from json")?
        as u32;
    let coords_value = views_json["coords"]
        .as_array()
        .ok_or("Failed to parse coords from json")?;
    
    let mut coords: Vec<CoordsTuple> = Vec::new();
    for coords_str in coords_value {
        let s = coords_str
            .as_str()
            .ok_or("Failed to convert a coords entry into a String")?
            .strip_prefix("i:")
            .ok_or("Failed to strip the 'i:' prefix")?;
        let (src_part, dst_part) = s
            .split_once('>')
            .ok_or("Failed to split the src and dst parts")?;
        let (src_pos, size) = src_part
            .split_once('+')
            .ok_or("Failed to split the src and size parts")?;

        let src_pos: Vec<&str> = src_pos.split(',').collect();
        let size: Vec<&str> = size.split(',').collect();
        let dst_pos: Vec<&str> = dst_part.split(',').collect();

        let src_x = src_pos[0].parse::<u32>()?;
        let src_y = src_pos[1].parse::<u32>()?;
        let w = size[0].parse::<u32>()?;
        let h = size[1].parse::<u32>()?;
        let dst_x = dst_pos[0].parse::<u32>()?;
        let dst_y = dst_pos[1].parse::<u32>()?;
        coords.push((src_x, src_y, w, h, dst_x, dst_y));
    }

    Ok(Views {
        width,
        height,
        coords,
    })
}

fn descramble(img: &DynamicImage, views: &Views) -> RgbaImage {
    let mut orig = ImageBuffer::new(views.width, views.height);
    for (src_x, src_y, w, h, dst_x, dst_y) in views.coords.iter() {
        let tile = img.crop_imm(*src_x, *src_y, *w, *h);
        orig.copy_from(&tile, *dst_x, *dst_y);  // TODO use GenericImageView::view
    }
    orig
}

fn main() {
    // Parse args or print help
    fn print_usage(args: Vec<String>) {
        println!("Usage: {} https://kirapo.jp/*/viewer\n\tThe argument is the url of the comic you need to download. Ends with /viewer", args[0]);
    }
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 || args[1] == "--help" || args[1] == "-h" {
        print_usage(args);
        std::process::exit(1);
    }
    let re = regex::Regex::new(r"https://kirapo\.jp/.*/viewer$").unwrap();
    if !re.is_match(&args[1]) {
        println!("Invalid url: {}", args[1]);
        print_usage(args);
        std::process::exit(1);
    }
    let url = format!("{}/data/", args[1].strip_suffix("/viewer").unwrap()).to_string();
    let id = args[1]
        .strip_suffix("/viewer")
        .unwrap()
        .rfind('/')
        .unwrap();
    let p = args[1].rfind('/').unwrap();
    let id: u32 = args[1][id+1..p].parse().unwrap();

    // Download the images
    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36")
        .build()
        .unwrap();
    let mut imgs: Vec<image::DynamicImage> = Vec::new();
    let mut views: Vec<Views> = Vec::new();
    for i in 1.. {
        print!("\rDownloading image {:04}...", i);
        std::io::stdout().flush().unwrap();
        
        // Download the image
        let img_url = format!("{}{:04}.jpg", url, i);
        let img = client.get(&img_url).send();
        if img.is_err() {
            eprintln!("Error while downloading an image {}: {}", i, img.as_ref().unwrap().status());
            std::process::exit(1);
        }
        let img = img.unwrap();
        if img.status() == 404 {
            break;
        }
        if img.status() != 200 {
            eprintln!("Error while downloading an image {}: {}", i, img.status());
            std::process::exit(1);
        }
        let buffer = img
            .bytes()
            .unwrap();
        let img = image::load_from_memory(&buffer)
            .unwrap();
        imgs.push(img);

        // Parse the json
        let json_url = format!("{}{:04}.ptimg.json", url, i);
        let json_resp = client.get(&json_url).send();
        if json_resp.is_err() {
            eprintln!("Error: {}", json_resp.as_ref().unwrap().status());
            std::process::exit(1);
        }
        let json_str = json_resp.unwrap().text().unwrap();
        let view = parse_json(&json_str).unwrap();
        views.push(view);
    }
    println!("\n{} images downloaded. Descrambling...", imgs.len());

    // Descramble the images
    let mut descrambled_imgs: Vec<image::RgbaImage> = Vec::new();
    for (img, view) in imgs.iter().zip(views.iter()) {
        let orig = descramble(img, view);
        descrambled_imgs.push(orig);
    }

    // Save the images
    // make a directory
    let dir = format!("./{}", id);
    std::fs::create_dir(&dir).unwrap();
    println!("Saving images into {}...", dir);
    for (img, i) in descrambled_imgs.iter().zip(1..) {
        print!("\rSaving image {:04} (of {:04})...", i, imgs.len());
        std::io::stdout().flush().unwrap();
        let path = format!("{}/{}.png", dir, i);
        img.save(path).unwrap();
    }

    println!("\nDone.");
}
