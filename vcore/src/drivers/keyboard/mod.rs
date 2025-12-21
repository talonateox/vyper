use spin::Mutex;

use crate::info;

const BUFFER_SIZE: usize = 256;

static KEYBOARD: Mutex<Keyboard> = Mutex::new(Keyboard::new());

struct Keyboard {
    buffer: [u8; BUFFER_SIZE],
    read_pos: usize,
    write_pos: usize,
    shift_pressed: bool,
    caps_lock: bool,
}

impl Keyboard {
    const fn new() -> Self {
        Self {
            buffer: [0; BUFFER_SIZE],
            read_pos: 0,
            write_pos: 0,
            shift_pressed: false,
            caps_lock: false,
        }
    }

    fn push(&mut self, c: u8) {
        let next_write = (self.write_pos + 1) % BUFFER_SIZE;
        if next_write != self.read_pos {
            self.buffer[self.write_pos] = c;
            self.write_pos = next_write;
        }
    }

    fn pop(&mut self) -> Option<u8> {
        if self.read_pos == self.write_pos {
            None
        } else {
            let c = self.buffer[self.read_pos];
            self.read_pos = (self.read_pos + 1) % BUFFER_SIZE;
            Some(c)
        }
    }

    fn is_empty(&self) -> bool {
        self.read_pos == self.write_pos
    }
}

const SCANCODE_LEFT_SHIFT: u8 = 0x2A;
const SCANCODE_RIGHT_SHIFT: u8 = 0x36;
const SCANCODE_CAPS_LOCK: u8 = 0x3A;

pub fn handle_scancode(scancode: u8) {
    info!("Scancode: 0x{:02x} ({})", scancode, scancode);

    let mut kb = KEYBOARD.lock();

    if scancode & 0x80 != 0 {
        let key = scancode & 0x7F;
        if key == SCANCODE_LEFT_SHIFT || key == SCANCODE_RIGHT_SHIFT {
            kb.shift_pressed = false;
        }
        return;
    }

    match scancode {
        SCANCODE_LEFT_SHIFT | SCANCODE_RIGHT_SHIFT => {
            kb.shift_pressed = true;
            return;
        }
        SCANCODE_CAPS_LOCK => {
            kb.caps_lock = !kb.caps_lock;
            return;
        }
        _ => {}
    }

    let ascii = b'a';

    info!("{}", scancode);

    let ascii = if kb.caps_lock && ascii.is_ascii_alphabetic() {
        if kb.shift_pressed {
            ascii.to_ascii_lowercase()
        } else {
            ascii.to_ascii_uppercase()
        }
    } else {
        ascii
    };

    if ascii != 0 {
        kb.push(ascii);
    }
}

pub fn read_char() -> Option<u8> {
    KEYBOARD.lock().pop()
}

pub fn has_input() -> bool {
    !KEYBOARD.lock().is_empty()
}
