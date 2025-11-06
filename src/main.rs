use std::{
    io::{Write, stdout},
    time::Duration,
};

use rand::Rng;
use raylib::{RaylibHandle, RaylibThread, camera::Camera2D, math::Vector2, prelude::*};

fn load_font(ram: &mut [u8]) {
    let font = [
        0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
        0x20, 0x60, 0x20, 0x20, 0x70, // 1
        0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
        0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
        0x90, 0x90, 0xF0, 0x10, 0x10, // 4
        0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
        0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
        0xF0, 0x10, 0x20, 0x40, 0x40, // 7
        0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
        0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
        0xF0, 0x90, 0xF0, 0x90, 0x90, // A
        0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
        0xF0, 0x80, 0x80, 0x80, 0xF0, // C
        0xE0, 0x90, 0x90, 0x90, 0xE0, // D
        0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
        0xF0, 0x80, 0xF0, 0x80, 0x80, // F
    ];
    ram.copy_from_slice(&font);
}

fn fetch(ram: &[u8], program_counter: &mut u16) -> Option<u16> {
    if *program_counter + 1 >= 4096 {
        return None;
    }
    let opcode = Some(u16::from_be_bytes(
        ram[*program_counter as usize..*program_counter as usize + 2]
            .try_into()
            .unwrap(),
    ));
    *program_counter += 2;
    opcode
}

fn decode_execute(
    ram: &mut [u8],
    display: &mut [[bool; 64]; 32],
    program_counter: &mut u16,
    index_register: &mut u16,
    stack: &mut Vec<u16>,
    delay_timer: &mut u8,
    sound_timer: &mut u8,
    general_purpose_registers: &mut [u8; 16],
    opcode: u16,
    rl: &mut RaylibHandle,
) {
    let x = ((opcode & 0x0F00) >> 8) as usize;
    let y = ((opcode & 0x00F0) >> 4) as usize;
    let n = (opcode & 0x000F) as u8;
    let nn = (opcode & 0x00FF) as u8;
    let nnn = opcode & 0x0FFF;

    match opcode {
        op if op & 0xFFFF == 0x00E0 => {
            for y in 0..display.len() {
                for x in 0..display[0].len() {
                    display[y][x] = false;
                }
            }
        }
        op if op & 0xF000 == 0x1000 => {
            *program_counter = nnn;
        }
        op if op & 0xFFFF == 0x00EE => {
            *program_counter = stack.pop().unwrap();
        }
        op if op & 0xF000 == 0x2000 => {
            stack.push(*program_counter);
            *program_counter = nnn;
        }
        op if op & 0xF000 == 0x3000 => {
            if general_purpose_registers[x] == nn {
                *program_counter += 2;
            }
        }
        op if op & 0xF000 == 0x4000 => {
            if general_purpose_registers[x] != nn {
                *program_counter += 2;
            }
        }
        op if op & 0xF00F == 0x5000 => {
            if general_purpose_registers[x] == general_purpose_registers[y] {
                *program_counter += 2;
            }
        }
        op if op & 0xF00F == 0x9000 => {
            if general_purpose_registers[x] != general_purpose_registers[y] {
                *program_counter += 2;
            }
        }
        op if op & 0xF000 == 0x6000 => {
            general_purpose_registers[x] = nn;
        }
        op if op & 0xF000 == 0x7000 => {
            general_purpose_registers[x] = general_purpose_registers[x].wrapping_add(nn);
        }
        op if op & 0xF00F == 0x8000 => {
            general_purpose_registers[x] = general_purpose_registers[y];
        }
        op if op & 0xF00F == 0x8001 => {
            general_purpose_registers[x] =
                general_purpose_registers[x] | general_purpose_registers[y];
        }
        op if op & 0xF00F == 0x8002 => {
            general_purpose_registers[x] =
                general_purpose_registers[x] & general_purpose_registers[y];
        }
        op if op & 0xF00F == 0x8003 => {
            general_purpose_registers[x] =
                general_purpose_registers[x] ^ general_purpose_registers[y];
        }
        op if op & 0xF00F == 0x8004 => {
            let (sum, is_overflowed) =
                general_purpose_registers[x].overflowing_add(general_purpose_registers[y]);
            general_purpose_registers[x] = sum;
            general_purpose_registers[0xF] = if is_overflowed { 1 } else { 0 };
        }
        op if op & 0xF00F == 0x8005 => {
            general_purpose_registers[0xF] =
                if general_purpose_registers[x] > general_purpose_registers[y] {
                    1
                } else {
                    0
                };
            general_purpose_registers[x] =
                general_purpose_registers[x].wrapping_sub(general_purpose_registers[y]);
        }
        op if op & 0xF00F == 0x8007 => {
            general_purpose_registers[0xF] =
                if general_purpose_registers[x] > general_purpose_registers[y] {
                    1
                } else {
                    0
                };
            general_purpose_registers[x] =
                general_purpose_registers[y].wrapping_sub(general_purpose_registers[x]);
        }
        op if op & 0xF00F == 0x8006 => {
            //Ambiguous instruction
            general_purpose_registers[0xF] = general_purpose_registers[x] & 0b0000_0001;
            general_purpose_registers[x] >>= 1;
        }
        op if op & 0xF00F == 0x800E => {
            //Ambiguous instruction
            general_purpose_registers[0xF] = general_purpose_registers[x] & 0b1000_0000;
            general_purpose_registers[x] <<= 1;
        }
        op if op & 0xF000 == 0xA000 => {
            *index_register = nnn;
        }
        op if op & 0xF000 == 0xB000 => {
            //Ambiguous instruction
            *program_counter = nnn + general_purpose_registers[0] as u16;
        }
        op if op & 0xF000 == 0xC000 => {
            general_purpose_registers[x] = rand::rng().random::<u8>() & nn
        }
        op if op & 0xF000 == 0xD000 => {
            let x_pos = (general_purpose_registers[x] as usize) % display[0].len();
            let y_pos = (general_purpose_registers[y] as usize) % display.len();

            let mut pixel_turned_off = false;
            for i in 0..n as usize {
                for j in 0..8 {
                    if y_pos + i >= display.len() || x_pos >= display[0].len() {
                        continue;
                    }
                    let new_pixel = (ram[*index_register as usize + i]) & (0x1 << (7 - j));
                    if new_pixel > 0 {
                        if display[y_pos + i][x_pos + j] == true {
                            pixel_turned_off = true;
                            display[y_pos + i][x_pos + j] = false;
                        } else {
                            display[y_pos + i][x_pos + j] = true;
                        }
                    }
                }
            }
            if pixel_turned_off {
                general_purpose_registers[0xF] = 1;
            } else {
                general_purpose_registers[0xF] = 0;
            }
        }
        op if op & 0xF0FF == 0xE09E => {
            let key = match general_purpose_registers[x] {
                0 => KeyboardKey::KEY_ONE,
                1 => KeyboardKey::KEY_TWO,
                2 => KeyboardKey::KEY_THREE,
                3 => KeyboardKey::KEY_FOUR,
                4 => KeyboardKey::KEY_Q,
                5 => KeyboardKey::KEY_W,
                6 => KeyboardKey::KEY_E,
                7 => KeyboardKey::KEY_R,
                8 => KeyboardKey::KEY_A,
                9 => KeyboardKey::KEY_S,
                10 => KeyboardKey::KEY_D,
                11 => KeyboardKey::KEY_F,
                12 => KeyboardKey::KEY_Z,
                13 => KeyboardKey::KEY_X,
                14 => KeyboardKey::KEY_C,
                15 => KeyboardKey::KEY_V,
                _ => panic!("Invalid key"),
            };

            if rl.is_key_down(key) {
                *program_counter += 2;
            }
        }
        op if op & 0xF0FF == 0xE0A1 => {
            let key = match general_purpose_registers[x] {
                0 => KeyboardKey::KEY_ONE,
                1 => KeyboardKey::KEY_TWO,
                2 => KeyboardKey::KEY_THREE,
                3 => KeyboardKey::KEY_FOUR,
                4 => KeyboardKey::KEY_Q,
                5 => KeyboardKey::KEY_W,
                6 => KeyboardKey::KEY_E,
                7 => KeyboardKey::KEY_R,
                8 => KeyboardKey::KEY_A,
                9 => KeyboardKey::KEY_S,
                10 => KeyboardKey::KEY_D,
                11 => KeyboardKey::KEY_F,
                12 => KeyboardKey::KEY_Z,
                13 => KeyboardKey::KEY_X,
                14 => KeyboardKey::KEY_C,
                15 => KeyboardKey::KEY_V,
                _ => panic!("Invalid key"),
            };

            if !rl.is_key_down(key) {
                *program_counter += 2;
            }
        }
        op if op & 0xF0FF == 0xF007 => {
            general_purpose_registers[x] = *delay_timer;
        }
        op if op & 0xF0FF == 0xF015 => {
            *delay_timer = general_purpose_registers[x];
        }
        op if op & 0xF0FF == 0xF018 => *sound_timer = general_purpose_registers[x],
        op if op & 0xF0FF == 0xF01E => {
            let (sum, is_overflowed) =
                (*index_register).overflowing_add(general_purpose_registers[x] as u16);
            *index_register = sum;
            general_purpose_registers[0xF] = if is_overflowed { 1 } else { 0 };
        }
        op if op & 0xF0FF == 0xF00A => {
            let key = match general_purpose_registers[x] {
                0 => KeyboardKey::KEY_ONE,
                1 => KeyboardKey::KEY_TWO,
                2 => KeyboardKey::KEY_THREE,
                3 => KeyboardKey::KEY_FOUR,
                4 => KeyboardKey::KEY_Q,
                5 => KeyboardKey::KEY_W,
                6 => KeyboardKey::KEY_E,
                7 => KeyboardKey::KEY_R,
                8 => KeyboardKey::KEY_A,
                9 => KeyboardKey::KEY_S,
                10 => KeyboardKey::KEY_D,
                11 => KeyboardKey::KEY_F,
                12 => KeyboardKey::KEY_Z,
                13 => KeyboardKey::KEY_X,
                14 => KeyboardKey::KEY_C,
                15 => KeyboardKey::KEY_V,
                _ => panic!("Invalid key"),
            };

            if !rl.is_key_down(key) {
                *program_counter -= 2;
            }
        }
        op if op & 0xF0FF == 0xF029 => *index_register = 0x50 + general_purpose_registers[x] as u16,
        op if op & 0xF0FF == 0xF033 => {
            let mut decimal = general_purpose_registers[x];
            ram[*index_register as usize] = decimal % 10;
            decimal /= 10;
            ram[*index_register as usize + 1] = decimal % 10;
            decimal /= 10;
            ram[*index_register as usize + 2] = decimal;
        }
        op if op & 0xF0FF == 0xF055 => {
            ram[*index_register as usize..=*index_register as usize + x]
                .copy_from_slice(&general_purpose_registers[0..=x]);
        }
        op if op & 0xF0FF == 0xF065 => {
            general_purpose_registers[0..=x]
                .copy_from_slice(&ram[*index_register as usize..=*index_register as usize + x]);
        }
        op => unimplemented!("{}", op),
    }
}

fn update_screen(rl: &mut RaylibHandle, thread: &RaylibThread, display: &mut [[bool; 64]; 32]) {
    let mut d = rl.begin_drawing(&thread);
    d.clear_background(Color::BLACK);
    for y in 0..display.len() {
        for x in 0..display[0].len() {
            if display[y][x] == true {
                d.draw_rectangle(
                    x as i32 * (window_width / 64),
                    y as i32 * (window_height / 32),
                    window_width / 64,
                    window_height / 32,
                    Color::WHITE,
                );
            }
        }
    }
}

const window_width: i32 = 1000;
const window_height: i32 = 500;

fn main() {
    let mut ram = [0 as u8; 4096];
    let mut display = [[false; 64]; 32];
    let mut program_counter = 0x200 as u16;
    let mut index_register = 0 as u16;
    let mut stack = Vec::new();
    let mut delay_timer = 0 as u8;
    let mut sound_timer = 0 as u8;
    let mut general_purpose_registers = [0 as u8; 16];

    load_font(&mut ram[0x50..=0x9f]);
    let rom = std::fs::read("./assets/roms/test_opcode.ch8").unwrap();
    ram[0x200..0x200 + rom.len()].copy_from_slice(&rom[..rom.len()]);

    let mut cpu_clock = std::time::Instant::now();
    let mut update_clock = std::time::Instant::now();

    let (mut rl, thread) = raylib::init()
        .size(window_width, window_height)
        .title("Hello, World")
        .build();

    while !rl.window_should_close() {
        if cpu_clock.elapsed().as_micros() > 1000 {
            cpu_clock = std::time::Instant::now();

            let opcode = fetch(&ram, &mut program_counter);
            if opcode.is_some() {
                decode_execute(
                    &mut ram,
                    &mut display,
                    &mut program_counter,
                    &mut index_register,
                    &mut stack,
                    &mut delay_timer,
                    &mut sound_timer,
                    &mut general_purpose_registers,
                    opcode.unwrap(),
                    &mut rl,
                );
            } else {
                break;
            }
            if program_counter >= 0x200 + rom.len() as u16 {
                break;
            }
        }
        if update_clock.elapsed().as_millis() > 16 {
            update_clock = std::time::Instant::now();
            if delay_timer > 0 {
                delay_timer -= 1;
            }
            if sound_timer > 0 {
                sound_timer -= 1;
            }

            update_screen(&mut rl, &thread, &mut display);
        }
    }

    println!("Program finished!");
}
