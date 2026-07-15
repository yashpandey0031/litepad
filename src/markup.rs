//! Inline formatting stored *in the text itself* (`**bold**`, `*italic*`, `__underline__`).
//!
//! Keeping the markers in the file means formatting survives save/load, applies only to
//! the range you selected, and the notes stay ordinary plain-text/Markdown files.

/// Which inline style a toolbar button / shortcut applies.
#[derive(Clone, Copy, PartialEq)]
pub enum Fmt {
    Bold,
    Italic,
    Underline,
}

impl Fmt {
    pub fn marker(self) -> &'static str {
        match self {
            Fmt::Bold => "**",
            Fmt::Italic => "*",
            Fmt::Underline => "__",
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

/// A run of text sharing one style. `marker` runs are the `**`/`*`/`__` characters
/// themselves, which the editor renders invisibly (transparent, near-zero width) so the
/// styled text reads cleanly while the markers stay in the file for save/load round-trips.
pub struct Seg {
    pub start: usize, // byte offsets into the full text
    pub end: usize,
    pub style: Style,
    pub marker: bool,
}

pub fn parse(text: &str) -> Vec<Seg> {
    let mut out = Vec::new();
    parse_into(text, 0, Style::default(), &mut out);
    out
}

fn parse_into(text: &str, off: usize, style: Style, out: &mut Vec<Seg>) {
    let mut i = 0usize;
    let mut lit = 0usize;

    while i < text.len() {
        let rest = &text[i..];

        // Longest marker first so `**` never gets read as two `*`.
        let candidate: Option<(&str, Style)> = if rest.starts_with("**") && !style.bold {
            Some((
                "**",
                Style {
                    bold: true,
                    ..style
                },
            ))
        } else if rest.starts_with("__") && !style.underline {
            Some((
                "__",
                Style {
                    underline: true,
                    ..style
                },
            ))
        } else if rest.starts_with('*') && !style.italic {
            Some((
                "*",
                Style {
                    italic: true,
                    ..style
                },
            ))
        } else {
            None
        };

        if let Some((marker, inner_style)) = candidate {
            let mlen = marker.len();
            let after = &text[i + mlen..];
            // A lone `*` (bullets, "2 * 3") must not open an italic run.
            let opens = marker != "*" || !after.starts_with(char::is_whitespace);

            if opens {
                if let Some(rel) = after.find(marker) {
                    let close = i + mlen + rel;
                    if close > i + mlen {
                        if i > lit {
                            out.push(Seg {
                                start: off + lit,
                                end: off + i,
                                style,
                                marker: false,
                            });
                        }
                        out.push(Seg {
                            start: off + i,
                            end: off + i + mlen,
                            style,
                            marker: true,
                        });
                        parse_into(&text[i + mlen..close], off + i + mlen, inner_style, out);
                        out.push(Seg {
                            start: off + close,
                            end: off + close + mlen,
                            style,
                            marker: true,
                        });
                        i = close + mlen;
                        lit = i;
                        continue;
                    }
                }
            }
        }

        i += rest.chars().next().map(char::len_utf8).unwrap_or(1);
    }

    if lit < text.len() {
        out.push(Seg {
            start: off + lit,
            end: off + text.len(),
            style,
            marker: false,
        });
    }
}

fn char_to_byte(s: &str, ci: usize) -> usize {
    s.char_indices().nth(ci).map(|(b, _)| b).unwrap_or(s.len())
}

/// Wrap the selected char range in `marker` — or unwrap it if already wrapped (toggle).
/// Returns the new selection as char indices.
pub fn toggle_wrap(text: &mut String, start_c: usize, end_c: usize, marker: &str) -> (usize, usize) {
    let sb = char_to_byte(text, start_c);
    let eb = char_to_byte(text, end_c);
    let mlen = marker.len();
    let mchars = marker.chars().count();

    let already =
        sb >= mlen && text[..sb].ends_with(marker) && text[eb..].starts_with(marker);

    if already {
        // Remove the trailing marker first so the leading one's offsets stay valid.
        text.replace_range(eb..eb + mlen, "");
        text.replace_range(sb - mlen..sb, "");
        (start_c - mchars, end_c - mchars)
    } else {
        text.insert_str(eb, marker);
        text.insert_str(sb, marker);
        (start_c + mchars, end_c + mchars)
    }
}

/// Byte ranges of http(s) URLs, with trailing punctuation trimmed.
pub fn find_urls(text: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(rel) = text[from..].find("http") {
        let start = from + rel;
        let rest = &text[start..];
        if rest.starts_with("http://") || rest.starts_with("https://") {
            let mut end = start;
            for (i, c) in rest.char_indices() {
                if c.is_whitespace()
                    || matches!(c, '<' | '>' | '"' | '|' | '\\' | '^' | '`' | '{' | '}')
                {
                    break;
                }
                end = start + i + c.len_utf8();
            }
            while end > start {
                let last = text[..end].chars().next_back().unwrap();
                if matches!(last, '.' | ',' | ')' | ']' | '}' | '!' | '?' | ':' | ';' | '\'') {
                    end -= last.len_utf8();
                } else {
                    break;
                }
            }
            if end > start + 8 {
                out.push((start, end));
            }
            from = end.max(start + 4);
        } else {
            from = start + 4;
        }
    }
    out
}

/// The URL at a given character index, if any.
pub fn url_at(text: &str, char_idx: usize) -> Option<String> {
    let byte = char_to_byte(text, char_idx);
    find_urls(text)
        .into_iter()
        .find(|&(s, e)| byte >= s && byte < e)
        .map(|(s, e)| text[s..e].to_string())
}

/// Drop markers for display purposes (sidebar titles, filenames).
pub fn strip_markers(s: &str) -> String {
    s.replace("**", "").replace("__", "").replace('*', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The styles applied to each character of `text`, for assertions.
    fn styles_of(text: &str) -> Vec<(String, Style, bool)> {
        parse(text)
            .into_iter()
            .map(|s| (text[s.start..s.end].to_string(), s.style, s.marker))
            .collect()
    }

    #[test]
    fn wraps_only_the_selection() {
        let mut t = String::from("hello brave world");
        // select "brave" (chars 6..11)
        let (s, e) = toggle_wrap(&mut t, 6, 11, "**");
        assert_eq!(t, "hello **brave** world");
        assert_eq!((s, e), (8, 13));
        assert_eq!(&t[8..13], "brave"); // selection still covers the same word
    }

    #[test]
    fn toggling_twice_removes_the_markers() {
        let mut t = String::from("hello brave world");
        let (s, e) = toggle_wrap(&mut t, 6, 11, "**");
        let (s2, e2) = toggle_wrap(&mut t, s, e, "**");
        assert_eq!(t, "hello brave world");
        assert_eq!((s2, e2), (6, 11));
    }

    #[test]
    fn only_the_wrapped_span_is_styled() {
        let segs = styles_of("plain **bold** plain");
        let bold: Vec<_> = segs
            .iter()
            .filter(|(_, st, marker)| st.bold && !marker)
            .map(|(t, _, _)| t.as_str())
            .collect();
        assert_eq!(bold, vec!["bold"]);
        // The surrounding text must NOT be bold — this was the reported bug.
        let unstyled: Vec<_> = segs
            .iter()
            .filter(|(_, st, marker)| !st.bold && !marker)
            .map(|(t, _, _)| t.as_str())
            .collect();
        assert_eq!(unstyled, vec!["plain ", " plain"]);
    }

    #[test]
    fn italic_and_underline_spans() {
        let segs = styles_of("a *it* b __un__ c");
        let it: Vec<_> = segs
            .iter()
            .filter(|(_, s, m)| s.italic && !m)
            .map(|(t, _, _)| t.as_str())
            .collect();
        let un: Vec<_> = segs
            .iter()
            .filter(|(_, s, m)| s.underline && !m)
            .map(|(t, _, _)| t.as_str())
            .collect();
        assert_eq!(it, vec!["it"]);
        assert_eq!(un, vec!["un"]);
    }

    #[test]
    fn nested_bold_italic() {
        let segs = styles_of("**bold *both* **");
        let both: Vec<_> = segs
            .iter()
            .filter(|(_, s, m)| s.bold && s.italic && !m)
            .map(|(t, _, _)| t.as_str())
            .collect();
        assert_eq!(both, vec!["both"]);
    }

    #[test]
    fn lone_asterisks_are_literal() {
        // Bullets and arithmetic must not turn into italics.
        for text in ["* item one\n* item two", "2 * 3 * 4"] {
            assert!(
                parse(text).iter().all(|s| !s.style.italic && !s.marker),
                "{text:?} should stay literal"
            );
        }
    }

    #[test]
    fn unmatched_marker_stays_literal() {
        assert!(parse("**not closed").iter().all(|s| !s.style.bold));
    }

    #[test]
    fn urls_are_detected_and_trimmed() {
        let t = "see https://example.com/a, ok";
        let urls = find_urls(t);
        assert_eq!(urls.len(), 1);
        let (s, e) = urls[0];
        assert_eq!(&t[s..e], "https://example.com/a"); // trailing comma trimmed
        assert_eq!(url_at(t, 10).as_deref(), Some("https://example.com/a"));
        assert_eq!(url_at(t, 0), None);
    }

    #[test]
    fn multibyte_selection_is_safe() {
        let mut t = String::from("héllo wörld");
        let (s, e) = toggle_wrap(&mut t, 6, 11, "**");
        assert_eq!(t, "héllo **wörld**");
        assert_eq!(&t[..], "héllo **wörld**");
        assert_eq!((s, e), (8, 13));
    }
}
