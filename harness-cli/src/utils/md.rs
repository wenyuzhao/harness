use std::io::IsTerminal;

use polars::prelude::*;

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

    pub fn add_dataframe(&mut self, df: &DataFrame) {
        let md_table = self.df_to_markdown(df, None);
        self.content.push_str(&md_table);
    }

    pub fn add_dataframe_with_ci(&mut self, df: &DataFrame, ci: &DataFrame) {
        let md_table = self.df_to_markdown(df, Some(ci));
        self.content.push_str(&md_table);
    }

    fn fmt_val(&self, v: &AnyValue) -> String {
        match v {
            AnyValue::Float32(v) => format!("{:.3}", v),
            AnyValue::Float64(v) => format!("{:.3}", v),
            AnyValue::UInt8(v) => format!("{}", v),
            AnyValue::UInt16(v) => format!("{}", v),
            AnyValue::UInt32(v) => format!("{}", v),
            AnyValue::UInt64(v) => format!("{}", v),
            AnyValue::Int8(v) => format!("{}", v),
            AnyValue::Int16(v) => format!("{}", v),
            AnyValue::Int32(v) => format!("{}", v),
            AnyValue::Int64(v) => format!("{}", v),
            AnyValue::Boolean(v) => format!("{}", v),
            _ => {
                if let Some(v) = v.get_str() {
                    v.to_string()
                } else {
                    format!("{:?}", v)
                }
            }
        }
    }

    fn df_to_markdown(&self, df: &DataFrame, ci: Option<&DataFrame>) -> String {
        let pad = |c: &str, n: usize| (0..n).map(|_| c).collect::<Vec<_>>().join("");
        let pad_start =
            |s: &str, w: usize, c: char| format!("{}{}", pad(&c.to_string(), w - s.len()), s);
        let pad_end =
            |s: &str, w: usize, c: char| format!("{}{}", s, pad(&c.to_string(), w - s.len()));
        // Collect cell strings by columns
        let mut cols = vec![];
        let mut col_align_r = vec![];
        for col in df.get_columns() {
            col_align_r.push(col.dtype().is_numeric());
            let mut c = vec![col.name().to_owned()];
            // Collect CI values for this column, and find max text width
            let mut ci_vals = vec![];
            if let Some(ci) = ci {
                if let Some(ci_col) = ci
                    .get_columns()
                    .iter()
                    .find(|c| col.dtype().is_numeric() && c.name() == col.name())
                {
                    for i in 0..col.len() {
                        ci_vals.push(self.fmt_val(&ci_col.get(i).unwrap()));
                    }
                }
            }
            let max_ci_w = ci_vals.iter().map(|s| s.len()).max().unwrap_or(0);
            // Collect cell strings for this column
            for i in 0..col.len() {
                let mut v = self.fmt_val(&col.get(i).unwrap());
                // Append CI
                if !ci_vals.is_empty() {
                    v += &format!(" ± {}", pad_start(&ci_vals[i], max_ci_w, ' '));
                }
                c.push(v);
            }
            cols.push(c);
        }
        // Get each column's max width
        let mut col_widths = vec![];
        for col in &cols {
            col_widths.push(col.iter().map(|s| s.len()).max().unwrap());
        }
        // Update cols with padded strings
        for (j, col) in cols.iter_mut().enumerate() {
            for i in 0..col.len() {
                col[i] = pad_end(&col[i].clone(), col_widths[j], ' ');
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
                    s += [":", " "][col_align_r[i] as usize];
                    s += &pad("-", *w);
                    s += [" ", ":"][col_align_r[i] as usize];
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

#[macro_export]
macro_rules! print_md {
    ($($arg:tt)*) => {
        $crate::utils::md::print_md(format!($($arg)*));
    };
}
