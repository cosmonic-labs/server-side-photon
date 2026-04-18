use photon_rs::channels;
use photon_rs::colour_spaces;
use photon_rs::conv;
use photon_rs::effects;
use photon_rs::filters;
use photon_rs::monochrome;
use photon_rs::noise;
use photon_rs::transform;
use photon_rs::PhotonImage;
use photon_rs::Rgb;
use serde::Deserialize;

#[derive(Deserialize, Debug, Default)]
pub struct TransformRequest {
    pub transform: String,
    #[serde(default)]
    pub params: TransformParams,
}

#[derive(Deserialize, Debug, Default)]
pub struct TransformParams {
    pub int_val: Option<i32>,
    pub float_val: Option<f64>,
    pub float_val2: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub filter_name: Option<String>,
}

pub fn apply_transform(
    mut img: PhotonImage,
    name: &str,
    params: &TransformParams,
) -> Result<PhotonImage, String> {
    match name {
        // ==================== Effects ====================
        "effects.adjust_brightness" => {
            effects::adjust_brightness(&mut img, params.int_val.unwrap_or(30) as i16);
        }
        "effects.adjust_contrast" => {
            effects::adjust_contrast(&mut img, params.float_val.unwrap_or(30.0) as f32);
        }
        "effects.colorize" => {
            effects::colorize(&mut img);
        }
        "effects.color_horizontal_strips" => {
            let num = params.int_val.unwrap_or(8) as u8;
            let color = Rgb::new(
                params.float_val.unwrap_or(255.0) as u8,
                params.float_val2.unwrap_or(0.0) as u8,
                0u8,
            );
            effects::color_horizontal_strips(&mut img, num, color);
        }
        "effects.color_vertical_strips" => {
            let num = params.int_val.unwrap_or(8) as u8;
            let color = Rgb::new(
                params.float_val.unwrap_or(255.0) as u8,
                params.float_val2.unwrap_or(0.0) as u8,
                0u8,
            );
            effects::color_vertical_strips(&mut img, num, color);
        }
        "effects.dec_brightness" => {
            effects::dec_brightness(&mut img, params.int_val.unwrap_or(30) as u8);
        }
        "effects.dither" => {
            effects::dither(&mut img, params.int_val.unwrap_or(2) as u32);
        }
        "effects.frosted_glass" => {
            effects::frosted_glass(&mut img);
        }
        "effects.halftone" => {
            effects::halftone(&mut img);
        }
        "effects.horizontal_strips" => {
            let num = params.int_val.unwrap_or(8) as u8;
            effects::horizontal_strips(&mut img, num);
        }
        "effects.inc_brightness" => {
            effects::inc_brightness(&mut img, params.int_val.unwrap_or(30) as u8);
        }
        "effects.multiple_offsets" => {
            effects::multiple_offsets(
                &mut img,
                params.int_val.unwrap_or(30) as u32,
                0usize,
                params.float_val.unwrap_or(2.0) as usize,
            );
        }
        "effects.normalize" => {
            effects::normalize(&mut img);
        }
        "effects.offset" => {
            effects::offset(&mut img, 0, params.int_val.unwrap_or(30) as u32);
        }
        "effects.offset_blue" => {
            effects::offset_blue(&mut img, params.int_val.unwrap_or(30) as u32);
        }
        "effects.offset_green" => {
            effects::offset_green(&mut img, params.int_val.unwrap_or(30) as u32);
        }
        "effects.offset_red" => {
            effects::offset_red(&mut img, params.int_val.unwrap_or(30) as u32);
        }
        "effects.oil" => {
            effects::oil(&mut img, params.int_val.unwrap_or(4), params.float_val.unwrap_or(55.0));
        }
        "effects.pixelize" => {
            effects::pixelize(&mut img, params.int_val.unwrap_or(10));
        }
        "effects.primary" => {
            effects::primary(&mut img);
        }
        "effects.solarize" => {
            effects::solarize(&mut img);
        }
        "effects.solarize_retimg" => {
            img = effects::solarize_retimg(&img);
        }
        "effects.tint" => {
            effects::tint(
                &mut img,
                params.float_val.unwrap_or(78.0) as u32,
                params.float_val2.unwrap_or(12.0) as u32,
                0,
            );
        }
        "effects.vertical_strips" => {
            let num = params.int_val.unwrap_or(8) as u8;
            effects::vertical_strips(&mut img, num);
        }

        // ==================== Convolution ====================
        "conv.box_blur" => {
            conv::box_blur(&mut img);
        }
        "conv.detect_45_deg_lines" => {
            conv::detect_45_deg_lines(&mut img);
        }
        "conv.detect_135_deg_lines" => {
            conv::detect_135_deg_lines(&mut img);
        }
        "conv.detect_horizontal_lines" => {
            conv::detect_horizontal_lines(&mut img);
        }
        "conv.detect_vertical_lines" => {
            conv::detect_vertical_lines(&mut img);
        }
        "conv.edge_detection" => {
            conv::edge_detection(&mut img);
        }
        "conv.edge_one" => {
            conv::edge_one(&mut img);
        }
        "conv.emboss" => {
            conv::emboss(&mut img);
        }
        "conv.gaussian_blur" => {
            conv::gaussian_blur(&mut img, params.int_val.unwrap_or(3));
        }
        "conv.identity" => {
            conv::identity(&mut img);
        }
        "conv.laplace" => {
            conv::laplace(&mut img);
        }
        "conv.noise_reduction" => {
            conv::noise_reduction(&mut img);
        }
        "conv.prewitt_horizontal" => {
            conv::prewitt_horizontal(&mut img);
        }
        "conv.sharpen" => {
            conv::sharpen(&mut img);
        }
        "conv.sobel_global" => {
            conv::sobel_global(&mut img);
        }
        "conv.sobel_horizontal" => {
            conv::sobel_horizontal(&mut img);
        }
        "conv.sobel_vertical" => {
            conv::sobel_vertical(&mut img);
        }

        // ==================== Filters ====================
        "filters.cali" => {
            filters::cali(&mut img);
        }
        "filters.dramatic" => {
            filters::dramatic(&mut img);
        }
        "filters.duotone_horizon" => {
            filters::duotone_horizon(&mut img);
        }
        "filters.duotone_lilac" => {
            filters::duotone_lilac(&mut img);
        }
        "filters.duotone_ochre" => {
            filters::duotone_ochre(&mut img);
        }
        "filters.duotone_violette" => {
            filters::duotone_violette(&mut img);
        }
        "filters.firenze" => {
            filters::firenze(&mut img);
        }
        "filters.golden" => {
            filters::golden(&mut img);
        }
        "filters.lix" => {
            filters::lix(&mut img);
        }
        "filters.lofi" => {
            filters::lofi(&mut img);
        }
        "filters.neue" => {
            filters::neue(&mut img);
        }
        "filters.obsidian" => {
            filters::obsidian(&mut img);
        }
        "filters.pastel_pink" => {
            filters::pastel_pink(&mut img);
        }
        "filters.ryo" => {
            filters::ryo(&mut img);
        }
        "filters.filter" => {
            let name = params.filter_name.as_deref().unwrap_or("oceanic");
            filters::filter(&mut img, name);
        }

        // ==================== Monochrome ====================
        "monochrome.b_grayscale" => {
            monochrome::b_grayscale(&mut img);
        }
        "monochrome.decompose_max" => {
            monochrome::decompose_max(&mut img);
        }
        "monochrome.decompose_min" => {
            monochrome::decompose_min(&mut img);
        }
        "monochrome.desaturate" => {
            monochrome::desaturate(&mut img);
        }
        "monochrome.g_grayscale" => {
            monochrome::g_grayscale(&mut img);
        }
        "monochrome.grayscale" => {
            monochrome::grayscale(&mut img);
        }
        "monochrome.grayscale_human_corrected" => {
            monochrome::grayscale_human_corrected(&mut img);
        }
        "monochrome.grayscale_shades" => {
            monochrome::grayscale_shades(&mut img, params.int_val.unwrap_or(4) as u8);
        }
        "monochrome.monochrome" => {
            monochrome::monochrome(
                &mut img,
                params.float_val.unwrap_or(50.0) as u32,
                params.float_val2.unwrap_or(100.0) as u32,
                params.int_val.unwrap_or(150) as u32,
            );
        }
        "monochrome.r_grayscale" => {
            monochrome::r_grayscale(&mut img);
        }
        "monochrome.sepia" => {
            monochrome::sepia(&mut img);
        }
        "monochrome.single_channel_grayscale" => {
            monochrome::single_channel_grayscale(&mut img, params.int_val.unwrap_or(0) as usize);
        }
        "monochrome.threshold" => {
            monochrome::threshold(&mut img, params.int_val.unwrap_or(128) as u32);
        }

        // ==================== Channels ====================
        "channels.alter_blue_channel" => {
            channels::alter_blue_channel(&mut img, params.int_val.unwrap_or(50) as i16);
        }
        "channels.alter_channel" => {
            channels::alter_channel(
                &mut img,
                params.int_val.unwrap_or(0) as usize,
                params.float_val.unwrap_or(50.0) as i16,
            );
        }
        "channels.alter_channels" => {
            channels::alter_channels(
                &mut img,
                params.int_val.unwrap_or(50) as i16,
                params.float_val.unwrap_or(0.0) as i16,
                params.float_val2.unwrap_or(0.0) as i16,
            );
        }
        "channels.alter_green_channel" => {
            channels::alter_green_channel(&mut img, params.int_val.unwrap_or(50) as i16);
        }
        "channels.alter_red_channel" => {
            channels::alter_red_channel(&mut img, params.int_val.unwrap_or(50) as i16);
        }
        "channels.alter_two_channels" => {
            channels::alter_two_channels(
                &mut img,
                params.int_val.unwrap_or(0) as usize,
                params.float_val.unwrap_or(50.0) as i16,
                1usize,
                params.float_val2.unwrap_or(50.0) as i16,
            );
        }
        "channels.invert" => {
            channels::invert(&mut img);
        }
        "channels.remove_blue_channel" => {
            channels::remove_blue_channel(&mut img, params.int_val.unwrap_or(0) as u8);
        }
        "channels.remove_channel" => {
            channels::remove_channel(
                &mut img,
                params.int_val.unwrap_or(0) as usize,
                params.float_val.unwrap_or(0.0) as u8,
            );
        }
        "channels.remove_green_channel" => {
            channels::remove_green_channel(&mut img, params.int_val.unwrap_or(0) as u8);
        }
        "channels.remove_red_channel" => {
            channels::remove_red_channel(&mut img, params.int_val.unwrap_or(0) as u8);
        }
        "channels.swap_channels" => {
            channels::swap_channels(
                &mut img,
                params.int_val.unwrap_or(0) as usize,
                params.float_val.unwrap_or(2.0) as usize,
            );
        }

        // ==================== Colour Spaces ====================
        "colour_spaces.darken_hsl" => {
            colour_spaces::darken_hsl(&mut img, params.float_val.unwrap_or(0.2) as f32);
        }
        "colour_spaces.darken_hsluv" => {
            colour_spaces::darken_hsluv(&mut img, params.float_val.unwrap_or(0.2) as f32);
        }
        "colour_spaces.darken_hsv" => {
            colour_spaces::darken_hsv(&mut img, params.float_val.unwrap_or(0.2) as f32);
        }
        "colour_spaces.darken_lch" => {
            colour_spaces::darken_lch(&mut img, params.float_val.unwrap_or(0.2) as f32);
        }
        "colour_spaces.desaturate_hsl" => {
            colour_spaces::desaturate_hsl(&mut img, params.float_val.unwrap_or(0.2) as f32);
        }
        "colour_spaces.desaturate_hsluv" => {
            colour_spaces::desaturate_hsluv(&mut img, params.float_val.unwrap_or(0.2) as f32);
        }
        "colour_spaces.desaturate_hsv" => {
            colour_spaces::desaturate_hsv(&mut img, params.float_val.unwrap_or(0.2) as f32);
        }
        "colour_spaces.desaturate_lch" => {
            colour_spaces::desaturate_lch(&mut img, params.float_val.unwrap_or(0.2) as f32);
        }
        "colour_spaces.gamma_correction" => {
            colour_spaces::gamma_correction(
                &mut img,
                params.float_val.unwrap_or(2.2) as f32,
                params.float_val2.unwrap_or(2.2) as f32,
                params.int_val.map(|v| v as f32).unwrap_or(2.2),
            );
        }
        "colour_spaces.hue_rotate_hsl" => {
            colour_spaces::hue_rotate_hsl(&mut img, params.float_val.unwrap_or(30.0) as f32);
        }
        "colour_spaces.hue_rotate_hsluv" => {
            colour_spaces::hue_rotate_hsluv(&mut img, params.float_val.unwrap_or(30.0) as f32);
        }
        "colour_spaces.hue_rotate_hsv" => {
            colour_spaces::hue_rotate_hsv(&mut img, params.float_val.unwrap_or(30.0) as f32);
        }
        "colour_spaces.hue_rotate_lch" => {
            colour_spaces::hue_rotate_lch(&mut img, params.float_val.unwrap_or(30.0) as f32);
        }
        "colour_spaces.lighten_hsl" => {
            colour_spaces::lighten_hsl(&mut img, params.float_val.unwrap_or(0.2) as f32);
        }
        "colour_spaces.lighten_hsluv" => {
            colour_spaces::lighten_hsluv(&mut img, params.float_val.unwrap_or(0.2) as f32);
        }
        "colour_spaces.lighten_hsv" => {
            colour_spaces::lighten_hsv(&mut img, params.float_val.unwrap_or(0.2) as f32);
        }
        "colour_spaces.lighten_lch" => {
            colour_spaces::lighten_lch(&mut img, params.float_val.unwrap_or(0.2) as f32);
        }
        "colour_spaces.mix_with_colour" => {
            let color = Rgb::new(
                params.int_val.unwrap_or(255) as u8,
                params.float_val.unwrap_or(0.0) as u8,
                params.float_val2.unwrap_or(128.0) as u8,
            );
            colour_spaces::mix_with_colour(&mut img, color, 0.5);
        }
        "colour_spaces.saturate_hsl" => {
            colour_spaces::saturate_hsl(&mut img, params.float_val.unwrap_or(0.2) as f32);
        }
        "colour_spaces.saturate_hsluv" => {
            colour_spaces::saturate_hsluv(&mut img, params.float_val.unwrap_or(0.2) as f32);
        }
        "colour_spaces.saturate_hsv" => {
            colour_spaces::saturate_hsv(&mut img, params.float_val.unwrap_or(0.2) as f32);
        }
        "colour_spaces.saturate_lch" => {
            colour_spaces::saturate_lch(&mut img, params.float_val.unwrap_or(0.2) as f32);
        }
        "colour_spaces.hsl" => {
            colour_spaces::hsl(
                &mut img,
                "saturate",
                params.float_val.unwrap_or(0.2) as f32,
            );
        }
        "colour_spaces.hsluv" => {
            colour_spaces::hsluv(
                &mut img,
                "saturate",
                params.float_val.unwrap_or(0.2) as f32,
            );
        }
        "colour_spaces.hsv" => {
            colour_spaces::hsv(
                &mut img,
                "saturate",
                params.float_val.unwrap_or(0.2) as f32,
            );
        }
        "colour_spaces.lch" => {
            colour_spaces::lch(
                &mut img,
                "saturate",
                params.float_val.unwrap_or(0.2) as f32,
            );
        }

        // ==================== Transform ====================
        "transform.crop" => {
            let x1 = params.int_val.unwrap_or(0) as u32;
            let y1 = params.float_val.unwrap_or(0.0) as u32;
            let x2 = params.width.unwrap_or(img.get_width() / 2);
            let y2 = params.height.unwrap_or(img.get_height() / 2);
            img = transform::crop(&mut img, x1, y1, x2, y2);
        }
        "transform.fliph" => {
            transform::fliph(&mut img);
        }
        "transform.flipv" => {
            transform::flipv(&mut img);
        }
        "transform.padding_bottom" => {
            let pad = params.int_val.unwrap_or(20) as u32;
            let color = photon_rs::Rgba::new(255, 255, 255, 255);
            img = transform::padding_bottom(&img, pad, color);
        }
        "transform.padding_left" => {
            let pad = params.int_val.unwrap_or(20) as u32;
            let color = photon_rs::Rgba::new(255, 255, 255, 255);
            img = transform::padding_left(&img, pad, color);
        }
        "transform.padding_right" => {
            let pad = params.int_val.unwrap_or(20) as u32;
            let color = photon_rs::Rgba::new(255, 255, 255, 255);
            img = transform::padding_right(&img, pad, color);
        }
        "transform.padding_top" => {
            let pad = params.int_val.unwrap_or(20) as u32;
            let color = photon_rs::Rgba::new(255, 255, 255, 255);
            img = transform::padding_top(&img, pad, color);
        }
        "transform.padding_uniform" => {
            let pad = params.int_val.unwrap_or(20) as u32;
            let color = photon_rs::Rgba::new(255, 255, 255, 255);
            img = transform::padding_uniform(&img, pad, color);
        }
        "transform.resize" => {
            let w = params.width.unwrap_or(img.get_width() / 2);
            let h = params.height.unwrap_or(img.get_height() / 2);
            img = transform::resize(&img, w, h, transform::SamplingFilter::Lanczos3);
        }
        "transform.resample" => {
            let w = params.width.unwrap_or(img.get_width() / 2) as usize;
            let h = params.height.unwrap_or(img.get_height() / 2) as usize;
            img = transform::resample(&img, w, h);
        }
        "transform.seam_carve" => {
            let w = params.width.unwrap_or(img.get_width() * 9 / 10);
            let h = params.height.unwrap_or(img.get_height() * 9 / 10);
            img = transform::seam_carve(&img, w, h);
        }
        "transform.shearx" => {
            img = transform::shearx(&img, params.float_val.unwrap_or(15.0) as f32);
        }
        "transform.sheary" => {
            img = transform::sheary(&img, params.float_val.unwrap_or(15.0) as f32);
        }
        "transform.rotate" => {
            img = transform::rotate(&img, params.float_val.unwrap_or(90.0) as f32);
        }
        "transform.compress" => {
            img = transform::compress(&img, params.int_val.unwrap_or(75) as u8);
        }

        // ==================== Noise ====================
        "noise.add_noise_rand" => {
            noise::add_noise_rand(&mut img);
        }
        "noise.pink_noise" => {
            noise::pink_noise(&mut img);
        }

        _ => return Err(format!("unknown transform: {name}")),
    }

    Ok(img)
}
