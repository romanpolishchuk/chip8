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
    stack: &mut [u16; 16],
    delay_timer: &mut u8,
    sound_timer: &mut u8,
    general_purpose_registers: &mut [u8; 16],
    opcode: u16,
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
        // op if op & 0xFFFF == 0x00EE => {}
        // op if op & 0xFFFF == 0x00EE => {}
        op if op & 0xF000 == 0x6000 => {
            general_purpose_registers[x] = nn;
        }
        op if op & 0xF000 == 0x7000 => {
            general_purpose_registers[x] = general_purpose_registers[x].wrapping_add(nn);
        }
        op if op & 0xF000 == 0xA000 => {
            *index_register = nnn;
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
            update_screen(display);
        }
        op => unimplemented!("{}", op),
    }
}

fn update_screen(display: &mut [[bool; 64]; 32]) {
    print!("\x1B[2J\x1B[H");
    for y in 0..display.len() {
        let mut row = String::new();
        for x in 0..display[0].len() {
            if display[y][x] == true {
                row += "#";
            } else {
                row += ".";
            }
        }
        println!("{}", row);
    }
}

fn main() {
    let mut ram = [0 as u8; 4096];
    let mut display = [[false; 64]; 32];
    let mut program_counter = 0x200 as u16;
    let mut index_register = 0 as u16;
    let mut stack = [0 as u16; 16];
    let mut delay_timer = 0 as u8;
    let mut sound_timer = 0 as u8;
    let mut general_purpose_registers = [0 as u8; 16];

    load_font(&mut ram[0x50..=0x9f]);
    let rom = std::fs::read("./assets/roms/IBM_Logo.ch8").unwrap();
    ram[0x200..0x200 + rom.len()].copy_from_slice(&rom[..rom.len()]);

    loop {
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
            );
        } else {
            break;
        }
        if program_counter >= 0x200 + rom.len() as u16 {
            break;
        }
    }
    println!("Program finished");
}
