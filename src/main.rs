extern crate image;
extern crate rand;

use image::{GenericImage, GenericImageView};
use rand::prelude::*;
use std::time::{Duration, SystemTime};
use std::thread::sleep;


// Using image crate
fn main() {
    let src = to_black_and_white(&image::open("images/girl_with_pearl.jpg").unwrap());
    let mut dest = create_average_background_image(&src);
    println!("{:?}", get_rmse(&dest, &src));
    for i in 0..200 {
        println!("Drawing shape number {:?}", i+1);
        dest = add_best_shape(&dest, &src);
        dest.save(format!("images/girl_iter1_step{:?}.jpg", i)).unwrap();
    }
}


fn create_average_background_image(img: &image::DynamicImage) -> image::DynamicImage {
    // TODO: Clean up this block. See if there is a more elegant way
    let dim = img.dimensions();
    let image_width = dim.0;
    let image_height = dim.1;
    let mut avg = [0 as u32, 0 as u32, 0 as u32];

    for pixel in img.pixels() {
        avg[0] += pixel.2[0] as u32;
        avg[1] += pixel.2[1] as u32;
        avg[2] += pixel.2[2] as u32;
    }
    avg[0] = avg[0] / (image_width * image_height);
    avg[1] = avg[1] / (image_width * image_height);
    avg[2] = avg[2] / (image_width * image_height);
    // TODO: Look into creating a buffer and converting that into image
    let mut dest = image::DynamicImage::new_rgb8(image_width, image_height);
    for x in  0..image_width {
        for y in 0..image_height {
            dest.put_pixel(x, y, image::Rgba{data: [avg[0] as u8, avg[1] as u8, avg[2] as u8, 255]});
        }
    }
    dest
}

fn to_black_and_white(img: &image::DynamicImage) -> image::DynamicImage {
    let dim = img.dimensions();
    let image_width = dim.0;
    let image_height = dim.1;
    let mut dest = image::DynamicImage::new_rgb8(image_width, image_height);
    for pixel in img.pixels() {
        let avg = pixel.2[0]/3 + pixel.2[1]/3 + pixel.2[2]/3;
        dest.put_pixel(pixel.0, pixel.1, image::Rgba{data: [avg, avg, avg, 255]});
    }
    dest
}

fn get_rmse(img1: &image::DynamicImage, img2: &image::DynamicImage) -> f32 {
    let now = SystemTime::now();
    let mut square_error = 0 as f32;
    for pixel in img1.pixels() {
        let pixel1 = pixel.2;
        let pixel2 = img2.get_pixel(pixel.0, pixel.1);
        // All the RGB values are in u8. We would prefer i32 to prevent overflows etc.
        // TODO: Make this all a little nicer looking?
        let r1 = pixel1[0] as f32;
        let r2 = pixel2[0] as f32;
        square_error += (r2-r2).powf(2.0);
        // Can ignore these differences as we are doing grayscale
        // TODO: Implement if we are doing color images
        let g1 = pixel1[1] as f32;
        let g2 = pixel2[1] as f32;
        square_error += (g1-g2).powf(2.0);
        let b1 = pixel1[2] as f32;
        let b2 = pixel2[2] as f32;
        square_error += (b1-b2).powf(2.0);
    }
    square_error /= img1.dimensions().0 as f32 * img1.dimensions().1 as f32;
    // println!("Getting RMSE took {:?}", now.elapsed());
    square_error.powf(0.5)
}

fn add_best_shape(img: &image::DynamicImage, src: &image::DynamicImage) -> image::DynamicImage {
    // Currently only grayscale rectangles, opimised by 100 hill climbs
    // Hill climb algo scammed from wikipedia
    let mut temp_image = img.clone();
    let entry_score = get_rmse(&img, src);

    let mut current_shape = get_start_point(&img);
    let mut step_sizes = [10.0, 10.0, 10.0, 10.0, 10.0];
    let acceleration = 1.2;
    let candidates = [-acceleration, -1.0/acceleration, 0.0, 1.0/acceleration, acceleration];

    for n in 0..200 {
        let now = SystemTime::now();
        let mut rmse_count = 0;
        let mut draw_count = 0;
        temp_image = draw_shape(current_shape, &img, src);
        let before_score = get_rmse(&temp_image, &src);
        for i in 0..current_shape.len() {
            let mut best = 10;
            let mut best_score = std::i32::MAX as f32;
            for j in 0..candidates.len() {
                current_shape[i] = current_shape[i] + (step_sizes[i] * candidates[j]) as i32;
                let current_img = draw_shape(current_shape, &temp_image, src);
                draw_count = draw_count + 1;
                let temp = get_rmse(&current_img, src);
                rmse_count = rmse_count + 1;
                current_shape[i] = current_shape[i] - (step_sizes[i] * candidates[j]) as i32;
                if temp < best_score {
                    best_score = temp;
                    best = j;
                }
            }
            println!("best={:?}, best_score={:?}", best, best_score);
            if candidates[best] == 0.0 {
                step_sizes[i] = step_sizes[i]/acceleration;
            } else {
                current_shape[i] = current_shape[i] + (step_sizes[i] * candidates[best]) as i32;
                step_sizes[i] = step_sizes[i] * candidates[best];
            }
        }
        if before_score - get_rmse(&draw_shape(current_shape, &temp_image, src), src) > 5.0 {
            return draw_shape(current_shape, &img, src)
        }
        rmse_count = rmse_count + 1;
        draw_count = draw_count + 1;
        println!("rmse_count: {:?}, draw_count: {:?}, time: {:?}", rmse_count, draw_count, now.elapsed());
    }
    let final_img = draw_shape(current_shape, &img, src);
    if get_rmse(&final_img, src) > entry_score {
        final_img
    } else {
        img.clone()
    }
}

fn get_start_point(img: &image::DynamicImage) -> [i32; 5] {
    // generate random start point
    let image_width = img.dimensions().0;
    let image_height = img.dimensions().1;

    let mut rng = rand::thread_rng();
    // NOTE: Keeping color as i32 for ease of code. Might need to constrain it somewhere
    // though logically, no optimisation method should bother going outside the range
    // PS. Note about constraints applies to all the variables...
    let x1: i32 = rng.gen_range(0, image_width as i32);
    let y1: i32 = rng.gen_range(0, image_height as i32);
    let x2: i32 = rng.gen_range(0, image_width as i32);
    let y2: i32 = rng.gen_range(0, image_height as i32);
    let color: i32 = rng.gen_range(0, 255);
    [x1, y1, x2, y2, color]
}

fn draw_shape(shape: [i32;5], img: &image::DynamicImage, src: &image::DynamicImage) -> image::DynamicImage {
    let image_width = img.dimensions().0 as i32;
    let image_height = img.dimensions().1 as i32;
    let mut new_img = img.clone();

    let mut minx = std::cmp::min(shape[0], shape[2]);
    let mut maxx = std::cmp::max(shape[0], shape[2]);
    let mut miny = std::cmp::min(shape[1], shape[3]);
    let mut maxy = std::cmp::max(shape[1], shape[3]);
    let mut clr = shape[4] as u8;
    // contstraining shape
    if minx < 0 { minx = 0; }
    if maxx >= image_width { maxx = image_width-1; }
    if miny < 0 { miny = 0; }
    if maxy >= image_height { maxy = image_height-1; }

    // draw the shape
    for x in minx..maxx+1 {
        for y in miny..maxy+1 {
            new_img.put_pixel(x as u32, y as u32, image::Rgba{data: [clr, clr, clr, 255]});
        }
    }
    new_img
}

fn image_to_vector(image: &image::DynamicImage) -> Vec<[u8; 3]> {
    let mut pixels = Vec::new();
    for pixel in image.pixels() {
        pixels.push([pixel.2[0], pixel.2[1], pixel.2[2]]);
    }
    pixels
}
