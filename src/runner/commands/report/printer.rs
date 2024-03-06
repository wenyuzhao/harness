use std::io::IsTerminal;

use polars::prelude::*;

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

    pub fn add_dataframe(&mut self, df: &DataFrame) {
        let md_table = self.df_to_markdown(df);
        self.content.push_str(&md_table);
    }

    fn fmt_value(&self, v: &AnyValue) -> (String, bool) {
        match v {
            AnyValue::Float32(v) => (format!("{:.3}", v), true),
            AnyValue::Float64(v) => (format!("{:.3}", v), true),
            AnyValue::UInt8(v) => (format!("{}", v), true),
            AnyValue::UInt16(v) => (format!("{}", v), true),
            AnyValue::UInt32(v) => (format!("{}", v), true),
            AnyValue::UInt64(v) => (format!("{}", v), true),
            AnyValue::Int8(v) => (format!("{}", v), true),
            AnyValue::Int16(v) => (format!("{}", v), true),
            AnyValue::Int32(v) => (format!("{}", v), true),
            AnyValue::Int64(v) => (format!("{}", v), true),
            AnyValue::Boolean(v) => (format!("{}", v), true),
            _ => {
                if let Some(v) = v.get_str() {
                    (v.to_string(), false)
                } else {
                    (format!("{:?}", v), true)
                }
            }
        }
    }

    fn df_to_markdown(&self, df: &DataFrame) -> String {
        // Collect cell strings by columns
        let mut cols = vec![];
        let mut col_align_r = vec![];
        for col in df.get_columns() {
            let mut c = vec![col.name().to_owned()];
            for i in 0..col.len() {
                let (v, align_right) = self.fmt_value(&col.get(i).unwrap());
                c.push(v);
                if i == 0 {
                    col_align_r.push(align_right);
                }
            }
            cols.push(c);
        }
        // Get each column's max width
        let mut col_widths = vec![];
        for col in &cols {
            col_widths.push(col.iter().map(|s| s.len()).max().unwrap());
        }
        // Update cols with padded strings
        let pad = |c: &str, n: usize| (0..n).map(|_| c).collect::<Vec<_>>().join("");
        for (j, col) in cols.iter_mut().enumerate() {
            for i in 0..col.len() {
                let s = col[i].clone();
                col[i] += &pad(" ", col_widths[j] - s.len());
            }
        }
        // Construct markdown table string, row by row
        let build_row = |values: Option<Vec<&str>>, align: bool| {
            let mid = if let Some(values) = values {
                values.join(" | ")
            } else if !align {
                (0..cols.len())
                    .map(|i| pad("-", col_widths[i]))
                    .collect::<Vec<_>>()
                    .join(" | ")
            } else {
                let mut s = "|".to_string();
                for (i, w) in col_widths.iter().enumerate() {
                    if !col_align_r[i] {
                        s += ":";
                    } else {
                        s += " ";
                    }
                    s += &pad("-", *w);
                    if col_align_r[i] {
                        s += ":";
                    } else {
                        s += " ";
                    }
                    s += "|";
                }
                return s + "\n";
            };
            "| ".to_string() + mid.as_str() + " |\n"
        };
        let rows = cols[0].len();
        let mut md = "".to_string();
        if self.is_tty() {
            md += &build_row(None, false);
        }
        for i in 0..rows {
            md += &build_row(Some(cols.iter().map(|c| c[i].as_str()).collect()), false);
            if i == 0 || (self.is_tty() && i == rows - 1) {
                md += &build_row(None, i == 0);
            }
        }
        return md;
    }
}
