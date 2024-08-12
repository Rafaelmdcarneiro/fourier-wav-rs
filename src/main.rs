extern crate alsa;
extern crate byteorder;
extern crate framebuffer;

mod wav_loader;
mod fb;

use std::env;
use std::{u8, i16, i32};
use std::f32::consts::PI;
use std::io::Write;
use std::ffi::CString;

use alsa::{Direction, ValueOr};
use alsa::pcm::{PCM, HwParams, Format, Access};

use wav_loader::{Wav, WavData};
use fb::FbPainter;

fn main() {
    let mut args = env::args();
    assert!(args.len() > 1);
    let path_to_audio = args.nth(1).unwrap();

    //Sound card settings
    let pcm = PCM::open(&*CString::new("default").unwrap(), Direction::Playback, false).unwrap();
    let hwp = HwParams::any(&pcm).unwrap();

    let wav = Wav::from_file(&path_to_audio).unwrap();
    let samples = match wav.data {
        WavData::U8(ref data) => u8_to_floats(&data),
        WavData::I16(ref data) => i16_to_floats(&data),
        WavData::I32(ref data) => i32_to_floats(&data),
        _ => panic!("Unhandled audio format"),
    };

    hwp.set_channels(wav.num_channels as u32).unwrap();
    hwp.set_rate(wav.sample_rate, ValueOr::Nearest).unwrap();
    hwp.set_format(Format::FloatLE).unwrap();
    hwp.set_access(Access::RWInterleaved).unwrap();
    pcm.hw_params(&hwp).unwrap();

    let mut painter = FbPainter::new();
    let mut io = pcm.io_f32().unwrap();
    for chunk in samples.chunks(512) {
        io.writei(chunk);
        let (c, s) = fft(chunk, chunk.len());

        //Only use the real part, since imaginary is a duplicate
        let mut magnitudes = Vec::new();
        for i in 0 .. c.len() / 2 { 
            magnitudes.push((c[i].powi(2) + s[i].powi(2)).sqrt() / (c.len() as f32 / 2.0));
        }
        painter.update(&magnitudes);
    }
}

fn hanning_window(index: usize, total: usize) -> f32 {
    0.5 * (1.0 - ((2.0 * index as f32 * PI) / (total as f32 - 1.0)).cos())
}

fn fft(samples: &[f32], len: usize) -> (Vec<f32>, Vec<f32>) {
    if len == 1 {
        (vec![samples[0]], vec![0.0])
    } else {
        let mut output_re = vec![0.0; len];
        let mut output_im = vec![0.0; len];

        let mut e_samples = Vec::new();
        let mut o_samples = Vec::new();

        for i in 0 .. len {
            if i % 2 == 0 {
                e_samples.push(samples[i]);
            } else {
                o_samples.push(samples[i]);
            }
        }

        let (even_re, even_im) = fft(&e_samples, len / 2);
        let (uneven_re, uneven_im) = fft(&o_samples, len / 2);

        for i in 0 .. len / 2 {
            let val = -2.0 * PI * (i as f32 / len as f32);

            output_re[i] = even_re[i] + val.cos() * uneven_re[i] - val.sin() * uneven_im[i];
            output_im[i] = even_im[i] + val.cos() * uneven_im[i] + val.sin() * uneven_re[i];

            output_re[i + len / 2] = even_re[i] - val.cos() * uneven_re[i] + val.sin() * uneven_im[i];
            output_im[i + len / 2] = even_im[i] - val.cos() * uneven_im[i] - val.sin() * uneven_re[i];
        }
        (output_re, output_im)
    }
}

fn dft(samples: &[f32], bin_count: usize, sample_rate: u32) {
    assert!(bin_count <= sample_rate as usize / 2);

    //wave with frequency f = sin(f * 2 * pi)
    
    let step_size = 1.0 / sample_rate as f32;
    let freq_step = sample_rate as f32 / 2.0 / bin_count as f32;

    for bin in 0 .. bin_count {
        let mut sum_sin = 0.0;
        let mut sum_cos = 0.0;
        for (i, sample) in samples.iter().enumerate() {
            let input = freq_step * bin as f32 * 2.0 * PI * step_size * i as f32;
            //let window = hanning_window(bin, bin_count);
            //let sin_val = sample * window * input.sin();
            //let cos_val = sample * window * input.cos();
            sum_sin += sample * input.sin();
            sum_cos += sample * input.cos();
        }

        //let avg_sin = sum_sin / samples.len() as f32;
        //let avg_cos = sum_cos / samples.len() as f32;
        let magnitude = (sum_cos.powi(2) + sum_sin.powi(2)).sqrt();
        println!("{} {}", bin, magnitude);
    }
}

fn u8_to_floats(src: &[u8]) -> Vec<f32> {
    let half_max = (u8::MAX / 2) as isize;
    src.iter().map(|&sample| (sample as isize - half_max) as f32 / half_max as f32).collect()
}

fn i16_to_floats(src: &[i16]) -> Vec<f32> {
    src.iter().map(|&sample| sample as f32 / i16::MAX as f32).collect()
}

fn i32_to_floats(src: &[i32]) -> Vec<f32> {
    src.iter().map(|&sample| sample as f32 / i32::MAX as f32).collect()
}
