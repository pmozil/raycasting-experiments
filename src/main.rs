use crossterm::event::{read, Event, KeyCode};
use crossterm::{cursor, terminal, Command, ExecutableCommand, QueueableCommand};
use std::fmt;
use std::io::{stdout, Stdout, Write};
use std::vec::Vec;

const PIXEL_MAP: [[char; 5]; 2] = [
    ['█', '█', '▒', '░', '•'],
    ['•', '.', '.', '_', '_'],
];

struct Term<'a> {
    out: &'a mut Stdout,
}

impl<'a> Term<'a> {
    fn new(out: &'a mut Stdout) -> Term {
        Term { out }
    }

    fn write_buffer<Show: fmt::Display>(&mut self, output: &Show) {
        self.out.write_all(format!("{output}").as_bytes()).unwrap();
        self.flush().unwrap();
    }

    fn clear_all(&mut self) {
        self.queue(terminal::Clear(terminal::ClearType::All))
            .unwrap();
        self.queue(terminal::Clear(terminal::ClearType::Purge))
            .unwrap();
        self.queue(cursor::MoveTo(0, 0)).unwrap();
        self.flush().unwrap();
    }

    fn queue<Cmd: Command>(&mut self, cmd: Cmd) -> crossterm::Result<&mut Stdout> {
        self.out.queue(cmd)
    }

    fn execute<Cmd: Command>(&mut self, cmd: Cmd) -> crossterm::Result<&mut Stdout> {
        self.out.execute(cmd)
    }

    fn flush(&mut self) -> crossterm::Result<()> {
        self.out.flush()
    }
}

struct CharMatrix {
    width: usize,
    height: usize,
    buf: Vec<Vec<char>>,
}

impl CharMatrix {
    fn new(width: usize, height: usize) -> CharMatrix {
        CharMatrix {
            width,
            height,
            buf: vec![vec![' '; width]; height],
        }
    }
}

impl std::ops::Index<usize> for CharMatrix {
    type Output = Vec<char>;

    #[inline(always)]
    fn index<'a>(&'a self, idx: usize) -> &'a Self::Output {
        &self.buf[idx]
    }
}

impl std::ops::IndexMut<usize> for CharMatrix {
    #[inline(always)]
    fn index_mut<'a>(&'a mut self, idx: usize) -> &'a mut Vec<char> {
        &mut self.buf[idx]
    }
}

impl fmt::Display for CharMatrix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut result = String::new();
        for line in &self.buf {
            result.extend(line);
            result.push('\r');
            result.push('\n');
        }
        write!(f, "{}", result)
    }
}

struct Player<'a> {
    x: f32,
    y: f32,
    rot: f32,
    _fov: f32,
    movt_step: f32,
    map: &'a mut CharMatrix,
    screen: &'a mut CharMatrix,
}

impl<'a> Player<'a> {
    fn new(
        x: f32,
        y: f32,
        rot: f32,
        _fov: f32,
        movt_step: f32,
        map: &'a mut CharMatrix,
        screen: &'a mut CharMatrix,
    ) -> Player<'a> {
        map[y as usize][x as usize] = '█';
        Player {
            x,
            y,
            rot,
            _fov,
            movt_step,
            map,
            screen,
        }
    }

    fn mv(&mut self, dir: i8) {
        let dx: f32 = (-1.0) * (dir as f32) * self.rot.sin() * self.movt_step;
        let dy: f32 = (-1.0) * (dir as f32) * self.rot.cos() * self.movt_step;
        let new_pos: (usize, usize) = ((self.x + dx) as usize, (self.y + dy) as usize);
        let chr: char = self.map[new_pos.1][new_pos.0];
        if chr != '#' && new_pos.0 < self.map.width && new_pos.1 < self.map.width {
            if (self.x as usize, self.y as usize) != new_pos {
                self.map[new_pos.1][new_pos.0] = '█';
                self.map[self.y as usize][self.x as usize] = '.';
            }
            self.x = self.x + dx;
            self.y = self.y + dy;
        }
    }

    fn rotate(&mut self, rotation: f32) {
        self.rot = (self.rot + rotation) % (2.0 * std::f32::consts::PI);
        self.x = self.x.floor();
        self.y = self.y.floor();
    }

    fn render(&mut self) {
        let drot = self._fov / (self.screen.width as f32);
        let mut pos = self.rot + self._fov;
        for i in 0..self.screen.width {
            pos += drot;
            let (coeff_x, coeff_y) = (pos.sin(), pos.cos());
            let (mut x, mut y, mut steps) = (self.x, self.y, 0);
            while 0.0 <= x
                && x < self.map.width as f32
                && 0.0 <= y
                && y < self.map.height as f32
                && self.map[y as usize][x as usize] != '#'
            {
                x += coeff_x * self.movt_step / 6.0;
                y += coeff_y * self.movt_step / 6.0;
                steps += 1;
            }
            let dist = steps / 15;
            let tiles_for_ceiling = std::cmp::min(dist * 10, self.screen.width / 5) / 4;
            for j in 0..self.screen.height {
                if j <= tiles_for_ceiling {
                    self.screen[j][i] = ' ';
                } else if j <= (self.screen.height - tiles_for_ceiling) {
                    self.screen[j][i] = PIXEL_MAP[0][std::cmp::min(dist as usize, 4)];
                } else {
                    self.screen[j][i] = PIXEL_MAP[1][std::cmp::min(dist as usize, 4)];
                }
            }
        }
    }

    fn print(&self, term: &mut Term) {
        term.write_buffer(&format!(
            "{}{}{}°",
            self.screen,
            self.map,
            self.rot / std::f32::consts::PI * 180.0
        ));
    }
}

fn main() -> crossterm::Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout: Stdout = stdout();

    let mut term: Term = Term::new(&mut stdout);
    let mut screen: CharMatrix = CharMatrix::new(160, 40);
    for i in 0..screen.height {
        for j in 0..screen.width {
            screen.buf[i][j] = '█';
        }
    }
    let mut map = CharMatrix::new(19, 20);
    map.buf = vec![
        vec![
            '#', '#', '#', '#', '#', '#', '#', '#', '#', '#', '.', '.', '.', '.', '#', '#', '#',
            '#', '#',
        ],
        vec![
            '#', '.', '.', '.', '.', '.', '.', '.', '.', '.', '.', '.', '.', '.', '.', '.', '.',
            '.', '#',
        ],
        vec![
            '#', '.', '.', '.', '.', '#', '.', '.', '.', '.', '.', '.', '.', '.', '.', '.', '.',
            '.', '#',
        ],
        vec![
            '#', '.', '.', '.', '.', '#', '.', '.', '.', '.', '.', '.', '.', '.', '.', '.', '.',
            '.', '#',
        ],
        vec![
            '#', '.', '.', '.', '.', '#', '.', '.', '.', '.', '.', '.', '.', '.', '.', '.', '.',
            '.', '#',
        ],
        vec![
            '#', '.', '.', '.', '.', '#', '.', '.', '.', '.', '.', '.', '.', '.', '.', '.', '.',
            '.', '#',
        ],
        vec![
            '#', '.', '.', '.', '.', '#', '#', '#', '#', '#', '#', '.', '.', '.', '.', '.', '.',
            '.', '#',
        ],
        vec![
            '.', '.', '.', '.', '.', '.', '.', '.', '.', '.', '#', '.', '.', '.', '.', '.', '.',
            '.', '#',
        ],
        vec![
            '.', '.', '.', '.', '.', '.', '.', '.', '.', '.', '#', '.', '.', '.', '.', '.', '.',
            '.', '#',
        ],
        vec![
            '#', '.', '.', '.', '.', '.', '.', '.', '.', '.', '#', '.', '.', '.', '.', '.', '.',
            '.', '#',
        ],
        vec![
            '#', '.', '.', '.', '.', '.', '.', '.', '.', '.', '#', '.', '.', '.', '.', '.', '.',
            '.', '#',
        ],
        vec![
            '#', '#', '#', '#', '#', '#', '#', '#', '#', '#', '#', '.', '.', '.', '.', '.', '.',
            '.', '#',
        ],
        vec![
            '#', '.', '.', '.', '.', '.', '.', '.', '.', '.', '#', '.', '.', '.', '.', '.', '.',
            '.', '#',
        ],
        vec![
            '#', '.', '.', '.', '.', '.', '.', '.', '.', '.', '#', '.', '.', '.', '.', '.', '.',
            '.', '#',
        ],
        vec![
            '#', '.', '.', '.', '.', '.', '.', '.', '.', '.', '#', '.', '.', '.', '.', '.', '.',
            '.', '#',
        ],
        vec![
            '#', '.', '.', '.', '.', '.', '.', '.', '.', '.', '#', '.', '.', '.', '.', '.', '.',
            '.', '#',
        ],
        vec![
            '#', '.', '.', '.', '.', '.', '.', '.', '.', '.', '#', '.', '.', '.', '.', '.', '.',
            '.', '#',
        ],
        vec![
            '#', '.', '.', '.', '.', '.', '.', '.', '.', '.', '#', '.', '.', '.', '.', '.', '.',
            '.', '#',
        ],
        vec![
            '#', '.', '.', '.', '.', '.', '.', '.', '.', '.', '.', '.', '.', '.', '.', '.', '.',
            '.', '.',
        ],
        vec![
            '#', '#', '#', '#', '#', '#', '#', '#', '#', '#', '#', '#', '#', '#', '#', '#', '#',
            '#', '#',
        ],
    ];
    let pi: f32 = std::f32::consts::PI;
    let mut player: Player = Player::new(3.0, 5.0, 0.0, pi / 1.5, 0.5, &mut map, &mut screen);

    term.execute(cursor::Hide).unwrap();
    term.queue(cursor::SavePosition).unwrap();
    loop {
        term.clear_all();
        player.render();
        player.print(&mut term);

        let event = read()?;

        match event {
            Event::Key(key) => match key.code {
                KeyCode::Char(chr) => match chr {
                    'w' => player.mv(1),
                    's' => player.mv(-1),
                    'a' => player.rotate(pi / 32.0),
                    'd' => player.rotate(-pi / 32.0),
                    _ => (),
                },
                KeyCode::Esc => break,
                _ => (),
            },
            _ => (),
        }
    }
    term.clear_all();
    term.write_buffer(&format!("Done!\n"));
    term.execute(cursor::Show).unwrap();
    crossterm::terminal::disable_raw_mode()?;
    term.flush().unwrap();
    Ok(())
}
