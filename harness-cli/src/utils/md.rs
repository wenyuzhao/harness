use std::io::IsTerminal;

pub fn print_md(s: impl AsRef<str>) {
    let mut printer = MarkdownPrinter::new();
    printer.add(s);
    printer.dump();
}

pub struct MarkdownPrinter {
    content: String,
}

impl MarkdownPrinter {
    pub fn new() -> Self {
        Self {
            content: String::new(),
        }
    }

    fn is_tty(&self) -> bool {
        std::io::stdout().is_terminal()
    }

    pub fn dump(&self) {
        if self.is_tty() {
            let mut skin = termimad::MadSkin::default();
            for i in 0..8 {
                skin.headers[i].align = termimad::Alignment::Left;
                skin.headers[i].add_attr(termimad::crossterm::style::Attribute::Bold);
                skin.headers[i].set_fg(termimad::crossterm::style::Color::Blue);
            }
            skin.headers[0].set_bg(termimad::crossterm::style::Color::Blue);
            skin.headers[0].add_attr(termimad::crossterm::style::Attribute::NoUnderline);
            skin.print_text(&self.content);
        } else {
            println!("{}", self.content);
        }
    }

    pub fn add(&mut self, s: impl AsRef<str>) {
        self.content.push_str(s.as_ref());
    }
}

#[macro_export]
macro_rules! print_md {
    ($($arg:tt)*) => {
        $crate::utils::md::print_md(format!($($arg)*));
    };
}
