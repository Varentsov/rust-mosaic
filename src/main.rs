extern crate image;
extern crate docopt;
extern crate rand;

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

use image::GenericImage;
use docopt::Docopt;

const USAGE: &'static str = "
Mosaic.

Usage:
    ./mosaic get <path_to_image>
    ./mosaic scan <folder>
    ./mosaic (-h | --help)

Options:
    -h --help   Show this screen.
";
const W_NUMB: u32 = 10;
const H_NUMB: u32 = 10;
const MY_IMAGES_DIR: &'static str = "images_db";



#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone)]
struct MyColor {
    r: u8,
    g: u8,
    b: u8,
}

impl MyColor {
    fn new(r: u8, g: u8, b: u8) -> MyColor {
        MyColor {
            r: r,
            g: g,
            b: b,
        }
    }
    fn distance(&self, other: &MyColor) -> f32 {
        let dist: f32 =
        ((self.r as f32 - other.r as f32).powi(2) +
        (self.g as f32 - other.g as f32).powi(2) +
        (self.b as f32 - other.b as f32).powi(2)).sqrt();
        dist
    }
}

fn average_color(file: &Path) -> Option<MyColor> {
    let im = match image::open(file) {
        Ok(file) => {file},
        Err(e) => {println!("cant opent image, {}", e);return None}
    };

    let size = im.dimensions();
    let wh = size.0 * size.1;

    let mut avg_color = (0f32, 0f32, 0f32);

    for x in 0..size.0 {
        for y in 0..size.1 {
            let pixel = im.get_pixel(x, y);
            avg_color = (
                (avg_color.0 + pixel.data[0] as f32/wh as f32),
                (avg_color.1 + pixel.data[1] as f32/wh as f32),
                (avg_color.2 + pixel.data[2] as f32/wh as f32)
           );
        }
    }
    let color = MyColor::new(avg_color.0 as u8, avg_color.1 as u8, avg_color.2 as u8);
    Some(color)
}

fn process_image(path: &Path, db: &mut HashMap<MyColor, Vec<PathBuf>>) {
    let parent_image = image::open(path).unwrap();
    let parent_size = parent_image.dimensions();

    let (wi, hi) = (parent_size.0/W_NUMB, parent_size.1/H_NUMB);
    let child_size = (wi*W_NUMB, hi*H_NUMB);

    let mut imgbuf = image::ImageBuffer::new(child_size.0, child_size.1);
    let mut tmp_piece_color: MyColor;
    let mut rand_gen = rand::thread_rng();

    for i in 0..hi {
        for j in 0..wi {
            let pixel = parent_image.get_pixel(j*W_NUMB,i*H_NUMB);
            tmp_piece_color = MyColor::new(
                pixel.data[0],
                pixel.data[1],
                pixel.data[2],
                );
            let nearest_img: MyColor = nearest_color(&tmp_piece_color, db);
            {
                let image_from = db.get(&nearest_img).unwrap();
                let index: usize = rand::sample(&mut rand_gen, 0..image_from.len(), 1)[0];
                imgbuf.copy_from(&image::open(&image_from[index]).unwrap(), j*W_NUMB, i*H_NUMB);
            }
        }
    }

    let ref mut fout = File::create(&Path::new("result.png")).unwrap();
    let _ = image::ImageRgba8(imgbuf).save(fout, image::PNG);
}

fn create_db() ->  Option<HashMap<MyColor, Vec<PathBuf>>> {
    let mut db: HashMap<MyColor, Vec<PathBuf>> = HashMap::new();

    for entry in fs::read_dir(MY_IMAGES_DIR).unwrap() {
        let entry = entry.unwrap();
        match average_color(&entry.path()) {
            Some(avg_color) => {
                        if db.contains_key(&avg_color) {
                            let mut key = db.get_mut(&avg_color).unwrap();
                            //println!("picture with the same color");
                            key.push(entry.path());
                        }
                        else {
                            db.insert(avg_color, vec!(entry.path()));
                        }
                    },
            None => {println!("can not calculate average color")},
        };
    }

    match db.capacity() {
        0 => {println!("There are no images in db. You need to scan folder with images"); println!("{}", USAGE); panic!();},
        _ => Some(db)
    }
}

fn collect_images(directory: &Path) {
    for entry in fs::read_dir(directory).unwrap() {
        let entry = entry.unwrap();
        if fs::metadata(entry.path()).unwrap().is_dir() {
            collect_images(&entry.path());
        }
        else {
            if entry.path().extension().unwrap() == "jpg" || entry.path().extension().unwrap() == "png" {
                //println!("{:?}", entry.path());
                match fs::metadata(&Path::new("images_db").join(entry.path().file_stem().unwrap()).with_extension("jpg")) {
                        Ok(_) => {},//println!("This name already used")},
                        Err(_) => {
                            match image::open(entry.path()) {
                                Ok(img) => {
                                            let ref mut fout = File::create(&Path::new("images_db")
                                            .join(entry.path().file_stem().unwrap())
                                            .with_extension("jpg")).unwrap();

                                            let imgbuf = img.resize_exact(W_NUMB, H_NUMB, image::FilterType::Lanczos3);
                                            let _ = imgbuf.save(fout, image::JPEG);
                                            println!("{}, moved to db folder", entry.path().to_str().unwrap());
                                            },
                            Err(e) => {println!("{}, fail open image: {}", entry.path().to_str().unwrap(), e)},
                            };
                        }
                };
            }
        }
    }
}

fn nearest_color(color: &MyColor, db: &mut HashMap<MyColor, Vec<PathBuf>>) -> MyColor {
    let mut nearest: &MyColor = color;
    let mut distance: f32;
    let mut min_distance: f32 = 10000_f32;
    {
        for key in db.keys() {
            distance = color.distance(&key);
            if min_distance > distance {
                nearest = key;
                min_distance = distance;
            }
        }
    }
    *nearest
}

fn main() {
    let args = Docopt::new(USAGE)
                      .and_then(|dopt| dopt.parse())
                      .unwrap_or_else(|e| e.exit());

    match fs::metadata(MY_IMAGES_DIR) {
        Ok(_) => {},
        Err(_) => {fs::create_dir(MY_IMAGES_DIR).unwrap();println!("Folder created: {}", MY_IMAGES_DIR);}
    };

    if args.get_bool("scan") {
        let img_folder = Path::new(args.get_str("<folder>"));
        collect_images(&img_folder);
        return;
    }

    let file_path = Path::new(args.get_str("<path_to_image>"));
    match fs::metadata(&file_path) {
        Ok(_) => {},
        Err(_) => {println!("File that you gave does not exist"); return},
    };

    let mut db: HashMap<MyColor, Vec<PathBuf>> = create_db().unwrap();

    process_image(&file_path, &mut db);
    println!("Allright. New image was successfully created.");
}
