use spin::Mutex;

const BUFFER_SIZE: usize = 256;

static KEYBOARD: Mutex<Keyboard> = Mutex::new(Keyboard::new());

struct Keyboard {
    buffer: [u8; BUFFER_SIZE],
    read_pos: usize,
    write_pos: usize,
    shift_pressed: bool,
    caps_lock: bool,
    release_next: bool,
}

impl Keyboard {
    const fn new() -> Self {
        Self {
            buffer: [0; BUFFER_SIZE],
            read_pos: 0,
            write_pos: 0,
            shift_pressed: false,
            caps_lock: false,
            release_next: false,
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

const SCANCODE_RELEASE: u8 = 0xF0;
const SCANCODE_LEFT_SHIFT: u8 = 0x12;
const SCANCODE_RIGHT_SHIFT: u8 = 0x59;
const SCANCODE_CAPS_LOCK: u8 = 0x58;

fn scancode_to_ascii(scancode: u8, shift: bool, caps: bool) -> Option<u8> {
    let (unshifted, shifted) = match scancode {
        0x16 => (b'1', b'!'),
        0x1E => (b'2', b'@'),
        0x26 => (b'3', b'#'),
        0x25 => (b'4', b'$'),
        0x2E => (b'5', b'%'),
        0x36 => (b'6', b'^'),
        0x3D => (b'7', b'&'),
        0x3E => (b'8', b'*'),
        0x46 => (b'9', b'('),
        0x45 => (b'0', b')'),
        0x4E => (b'-', b'_'),
        0x55 => (b'=', b'+'),
        0x66 => (8, 8),
        0x0D => (b'\t', b'\t'),

        0x15 => (b'q', b'Q'),
        0x1D => (b'w', b'W'),
        0x24 => (b'e', b'E'),
        0x2D => (b'r', b'R'),
        0x2C => (b't', b'T'),
        0x35 => (b'y', b'Y'),
        0x3C => (b'u', b'U'),
        0x43 => (b'i', b'I'),
        0x44 => (b'o', b'O'),
        0x4D => (b'p', b'P'),
        0x54 => (b'[', b'{'),
        0x5B => (b']', b'}'),
        0x5A => (b'\n', b'\n'),

        0x1C => (b'a', b'A'),
        0x1B => (b's', b'S'),
        0x23 => (b'd', b'D'),
        0x2B => (b'f', b'F'),
        0x34 => (b'g', b'G'),
        0x33 => (b'h', b'H'),
        0x3B => (b'j', b'J'),
        0x42 => (b'k', b'K'),
        0x4B => (b'l', b'L'),
        0x4C => (b';', b':'),
        0x52 => (b'\'', b'"'),
        0x0E => (b'`', b'~'),
        0x5D => (b'\\', b'|'),

        0x1A => (b'z', b'Z'),
        0x22 => (b'x', b'X'),
        0x21 => (b'c', b'C'),
        0x2A => (b'v', b'V'),
        0x32 => (b'b', b'B'),
        0x31 => (b'n', b'N'),
        0x3A => (b'm', b'M'),
        0x41 => (b',', b'<'),
        0x49 => (b'.', b'>'),
        0x4A => (b'/', b'?'),

        0x29 => (b' ', b' '),

        _ => return None,
    };

    let is_letter = unshifted.is_ascii_lowercase();
    let use_shifted = if is_letter { shift ^ caps } else { shift };

    Some(if use_shifted { shifted } else { unshifted })
}

pub fn handle_scancode(scancode: u8) {
    let mut kb = KEYBOARD.lock();

    if scancode == SCANCODE_RELEASE {
        kb.release_next = true;
        return;
    }

    if kb.release_next {
        kb.release_next = false;
        if scancode == SCANCODE_LEFT_SHIFT || scancode == SCANCODE_RIGHT_SHIFT {
            kb.shift_pressed = false;
        }
        return;
    }

    match scancode {
        SCANCODE_LEFT_SHIFT | SCANCODE_RIGHT_SHIFT => {
            kb.shift_pressed = true;
        }
        SCANCODE_CAPS_LOCK => {
            kb.caps_lock = !kb.caps_lock;
        }
        _ => {
            if let Some(ascii) = scancode_to_ascii(scancode, kb.shift_pressed, kb.caps_lock) {
                kb.push(ascii);
            }
        }
    }
}

pub fn read_char() -> Option<u8> {
    KEYBOARD.lock().pop()
}

pub fn has_input() -> bool {
    !KEYBOARD.lock().is_empty()
}
