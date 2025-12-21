use spin::Mutex;

const BUFFER_SIZE: usize = 256;

static KEYBOARD: Mutex<Keyboard> = Mutex::new(Keyboard::new());

struct Keyboard {
    buffer: [u8; BUFFER_SIZE],
    read_pos: usize,
    write_pos: usize,
}

impl Keyboard {
    const fn new() -> Self {
        Self {
            buffer: [0; BUFFER_SIZE],
            read_pos: 0,
            write_pos: 0,
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

static SCANCODE_MAP: [u8; 128] = [
    0, 27, b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0', b'-', b'=', 8, b'\t', b'q',
    b'w', b'e', b'r', b't', b'y', b'u', b'i', b'o', b'p', b'[', b']', b'\n', 0, b'a', b's', b'd',
    b'f', b'g', b'h', b'j', b'k', b'l', b';', b'\'', b'`', 0, b'\\', b'z', b'x', b'c', b'v', b'b',
    b'n', b'm', b',', b'.', b'/', 0, b'*', 0, b' ', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

pub fn handle_scancode(scancode: u8) {
    if scancode & 0x80 != 0 {
        return;
    }

    if let Some(&ascii) = SCANCODE_MAP.get(scancode as usize) {
        if ascii != 0 {
            KEYBOARD.lock().push(ascii);
        }
    }
}

pub fn read_char() -> Option<u8> {
    KEYBOARD.lock().pop()
}

pub fn has_input() -> bool {
    !KEYBOARD.lock().is_empty()
}
