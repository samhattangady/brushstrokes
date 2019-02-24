extern crate image;
extern crate rand;

use image::{GenericImage, GenericImageView};
use rand::prelude::*;
use std::time::{SystemTime};

fn main() {
    let src = to_black_and_white(&image::open("images/girl_with_pearl.jpg").unwrap());
    let mut dest = create_average_background_image(&src);
    for i in 0..100 {
        let now = SystemTime::now();
        println!("Drawing shape number {:?}", i+1);
        dest = add_best_shape(&dest, &src, 0.3);
        dest.save(format!("images/girl_iter2_step{:?}.jpg", i+1)).unwrap();
        println!("time for step: {:?}\n", now.elapsed());
    }
}

fn create_average_background_image(src: &image::DynamicImage) -> image::DynamicImage {
    let src_pixels = image_to_vector(src);
    let dim = src.dimensions();
    let image_width = dim.0;
    let image_height = dim.1;

    let mut avg_color = 0;
    for pixel in src_pixels {
        avg_color += (pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32) / 3;
    }
    avg_color = avg_color / (image_width * image_height);

    let mut dest = image::DynamicImage::new_rgb8(image_width, image_height);
    for x in  0..image_width {
        for y in 0..image_height {
            dest.put_pixel(x, y, image::Rgba{data: [avg_color as u8, avg_color as u8, avg_color as u8, 255]});
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

fn add_best_shape(img: &image::DynamicImage, src: &image::DynamicImage, alpha: f32) -> image::DynamicImage {
    // Currently only grayscale rectangles, opimised by 100 hill climbs
    // Hill climb algo scammed from wikipedia
    // Don't know what epsilon is though
    let image_width = src.dimensions().0 as usize;
    let src_pixels = image_to_vector(src);
    let entry_pixels = image_to_vector(img);

    let mut step_sizes = [10.0, 10.0, 10.0, 10.0, 10.0];
    let acceleration = 1.2;
    let candidates = [-acceleration, -1.0/acceleration, 0.0, 1.0/acceleration, acceleration];
    let mut current_shape = get_start_point(&img);
    let mut best_score = std::i32::MAX as f32;
    let mut step_score;

    for _ in 0..100 {
        step_score = best_score;
        best_score = std::i32::MAX as f32;
        for i in 0..current_shape.len() {
            let mut best = 10;
            best_score = std::i32::MAX as f32;
            for j in 0..candidates.len() {
                current_shape[i] = current_shape[i] + (step_sizes[i] * candidates[j]) as i32;
                let current_pixels = draw_shape(current_shape, &entry_pixels, alpha, image_width);
                let current_score = get_rmse(&current_pixels, &src_pixels);
                current_shape[i] = current_shape[i] - (step_sizes[i] * candidates[j]) as i32;
                if current_score < best_score {
                    best_score = current_score;
                    best = j;
                }
            }
            if candidates[best] == 0.0 {
                step_sizes[i] = step_sizes[i]/acceleration;
            } else {
                current_shape[i] = current_shape[i] + (step_sizes[i] * candidates[best]) as i32;
                step_sizes[i] = step_sizes[i] * candidates[best];
            }
        }
        // TODO (24 Feb 2019 sam) Figure out what this epsilon value is supposed to be
        if (step_score - best_score) < 0.000005 {
            return vector_to_image(draw_shape(current_shape, &entry_pixels, alpha, image_width), src);
        }
    }
    let final_img = draw_shape(current_shape, &entry_pixels, alpha, image_width);
    // NOTE(24 Feb 2019 sam): We might want to check the score here to make sure its improved
    // Currently it is assumed that it is improved
    vector_to_image(final_img, src)
}

fn get_start_point(img: &image::DynamicImage) -> [i32; 5] {
    // generate random start point
    let image_width = img.dimensions().0;
    let image_height = img.dimensions().1;

    let mut rng = rand::thread_rng();
    // NOTE (20 Feb 2019 sam): Keeping color as i32 for ease of code. Might need to constrain it somewhere
    // though logically, no optimisation method should bother going outside the range
    // PS. Note about constraints applies to all the variables...
    let x1: i32 = rng.gen_range(0, image_width as i32);
    let y1: i32 = rng.gen_range(0, image_height as i32);
    let x2: i32 = rng.gen_range(0, image_width as i32);
    let y2: i32 = rng.gen_range(0, image_height as i32);
    let color: i32 = rng.gen_range(0, 255);
    [x1, y1, x2, y2, color]
}

fn get_rmse(img1: &Vec<[u8; 3]>, img2: &Vec<[u8; 3]>) -> f32 {
    let mut square_error = 0.0 as f32;
    for pixels in img1.iter().zip(img2.iter()) {
        // TODO (24 Feb 2019 sam) See how this can be cleaned up
        let (p1, p2) = pixels;
        let r1 = p1[0] as f32;
        let g1 = p1[1] as f32;
        let b1 = p1[2] as f32;
        let r2 = p2[0] as f32;
        let g2 = p2[1] as f32;
        let b2 = p2[2] as f32;
        square_error += (r2-r1).powf(2.0);
        square_error += (g2-g1).powf(2.0);
        square_error += (b2-b1).powf(2.0);
    }
    square_error /= (3 * img1.len()) as f32;
    square_error.powf(0.5)
}

fn draw_shape(shape: [i32;5], img: &Vec<[u8; 3]>, alpha: f32, image_width: usize) -> Vec<[u8; 3]> {
    let mut new_pixels = img.clone();
    let image_height = img.len() / image_width;

    let minx = std::cmp::min(shape[0], shape[2]) as usize;
    let miny = std::cmp::min(shape[1], shape[3]) as usize;
    let mut maxx = std::cmp::max(shape[0], shape[2]) as usize;
    let mut maxy = std::cmp::max(shape[1], shape[3]) as usize;
    let clr = shape[4] as u8;
    // contstraining shape
    if maxx >= image_width { maxx = image_width-1; }
    if maxy >= image_height { maxy = image_height-1; }

    // draw the shape
    for x in minx..maxx+1 {
        for y in miny..maxy+1 {
            // FIXME (24 Feb 2019 sam): Currently only using red channel
            // Will break when we move to rgb
            let old_clr = img[y as usize*image_width + x][0];
            let new_clr = ((clr as f32 * alpha) + old_clr as f32 * (1.0-alpha)) as u8;
            new_pixels[y as usize*image_width + x] = [new_clr, new_clr, new_clr];
        }
    }
    new_pixels
}

fn image_to_vector(image: &image::DynamicImage) -> Vec<[u8; 3]> {
    let mut pixels = Vec::new();
    for pixel in image.pixels() {
        pixels.push([pixel.2[0], pixel.2[1], pixel.2[2]]);
    }
    pixels
}

fn vector_to_image(vector: Vec<[u8; 3]>, src: &image::DynamicImage) -> image::DynamicImage {
    let mut img = src.clone();
    let width = img.dimensions().0 as usize;
    let height = img.dimensions().1 as usize;
    for x in 0..width {
        for y in 0..height {
            let pixel = vector[y*width + x];
            img.put_pixel(x as u32, y as u32, image::Rgba{ data:[pixel[0], pixel[1], pixel[2], 255] });
        }
    }
    img
}
