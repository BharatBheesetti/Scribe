use std::sync::OnceLock;
use regex::Regex;

/// Simple fillers: um, uh, umm, hmm, er
/// NOTE: "ah" is NOT included -- it's a meaningful interjection ("Ah, I see")
fn re_simple_fillers() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)(?:,\s*)?\b(?:um|uh|umm|hmm+|er)\b(?:\s*,)?").unwrap()
    })
}

/// "like" detection -- broad pattern, context logic decides keep/remove
fn re_like() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)(?:,\s*)?\blike\b(?:\s*,)?").unwrap()
    })
}

/// "you know" detection
fn re_you_know() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)(?:,\s*)?\byou know\b(?:\s*,)?").unwrap()
    })
}

/// "I mean" detection
fn re_i_mean() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)(?:,\s*)?\bI mean\b(?:\s*,)?").unwrap()
    })
}

/// "sort of" detection -- context-sensitive
fn re_sort_of() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)(?:,\s*)?\bsort of\b(?:\s*,)?").unwrap()
    })
}

/// "kind of" detection -- context-sensitive
fn re_kind_of() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)(?:,\s*)?\bkind of\b(?:\s*,)?").unwrap()
    })
}

/// "basically" detection -- context-sensitive
fn re_basically() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)(?:,\s*)?\bbasically\b(?:\s*,)?").unwrap()
    })
}

/// Orphaned commas: ", ," sequences left by filler removal
fn re_orphaned_commas() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r",(\s*,)+").unwrap()
    })
}

/// Multiple spaces
fn re_multi_space() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\s{2,}").unwrap()
    })
}

// ---------------------------------------------------------------------------
// Helper Functions
// ---------------------------------------------------------------------------

/// Extract the lowercase word immediately after a byte offset in the text.
/// Returns None if no following word exists.
fn following_word(text: &str, match_end: usize) -> Option<String> {
    let after = &text[match_end..];
    let trimmed = after.trim_start();
    let trimmed = trimmed.trim_start_matches(',').trim_start();
    trimmed
        .split(|c: char| c.is_whitespace() || c == ',')
        .next()
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
}

/// Strip common English contraction suffixes from a word.
/// "that's" -> "that", "don't" -> "don", "they're" -> "they", etc.
/// Handles both ASCII apostrophe (') and Unicode right single quote (U+2019).
///
/// Uses exact suffix matching after stripping -- does NOT use starts_with().
fn strip_contraction(word: &str) -> String {
    let suffixes = [
        "\u{2019}ve", "'ve",
        "\u{2019}re", "'re",
        "\u{2019}ll", "'ll",
        "\u{2019}s", "'s",
        "\u{2019}t", "'t",
        "\u{2019}d", "'d",
    ];
    let lower = word.to_lowercase();
    for suffix in &suffixes {
        if let Some(stem) = lower.strip_suffix(suffix) {
            if !stem.is_empty() {
                return stem.to_string();
            }
        }
    }
    lower
}

// ---------------------------------------------------------------------------
// Language Guard
// ---------------------------------------------------------------------------

/// Returns true if filler removal should be applied for this language.
/// Only English fillers are defined -- applying them to other languages
/// causes destructive false positives (e.g., German "er" = "he").
fn should_apply_filler_removal(language: &str) -> bool {
    matches!(language.to_lowercase().as_str(), "en" | "english" | "auto")
}

// ---------------------------------------------------------------------------
// Filler Removal Functions
// ---------------------------------------------------------------------------

/// Words that naturally take a following comma (discourse markers, interjections).
/// When a simple filler is removed from between these words and the next clause,
/// the comma after the discourse marker is preserved.
const DISCOURSE_MARKERS: &[&str] = &[
    "well", "so", "yes", "no", "ok", "okay", "right", "sure",
    "first", "second", "third", "finally", "actually", "anyway",
    "however", "indeed", "still", "now", "look", "see", "hey",
    "hi", "please", "thanks", "great", "fine", "true", "oh",
];

fn remove_simple_fillers(text: &str) -> String {
    let re = re_simple_fillers();
    let mut result = String::with_capacity(text.len());
    let mut last_end = 0;

    for mat in re.find_iter(text) {
        let match_str = mat.as_str();
        let has_leading_comma = match_str.trim_start().starts_with(',')
            || text[..mat.start()].trim_end().ends_with(',');
        let has_trailing_comma = match_str.trim_end().ends_with(',')
            || text[mat.end()..].trim_start().starts_with(',');

        result.push_str(&text[last_end..mat.start()]);

        // Preserve structural comma only when the preceding word is a discourse
        // marker (e.g., "Well, um, OK" -> "Well, OK" but "I was, uh, thinking" -> "I was thinking")
        if has_leading_comma && has_trailing_comma
            && !text[..mat.start()].trim_end().is_empty()
        {
            let prev = preceding_word_simple(text, mat.start());
            let is_discourse_marker = prev
                .as_ref()
                .map(|w| DISCOURSE_MARKERS.contains(&w.to_lowercase().as_str()))
                .unwrap_or(false);
            if is_discourse_marker {
                if !result.trim_end().ends_with(',') {
                    result.push(',');
                }
            }
        }
        result.push(' ');
        last_end = mat.end();
    }
    result.push_str(&text[last_end..]);
    result
}

/// Extract the word immediately before the match start (ignoring commas/whitespace).
fn preceding_word_simple(text: &str, match_start: usize) -> Option<String> {
    let before = &text[..match_start];
    let trimmed = before.trim_end();
    let trimmed = trimmed.trim_end_matches(',').trim_end();
    trimmed
        .rsplit(|c: char| c.is_whitespace() || c == ',')
        .next()
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
}

/// Remove "like" ONLY when it appears in known filler positions:
/// 1. Comma-wrapped: "I was, like, thinking" -> "I was thinking"
/// 2. Sentence start with comma: "Like, I was thinking" -> "I was thinking"
///
/// Do NOT remove "like" in any other position. False negatives (leaving
/// filler "like" in) are far less harmful than false positives (removing
/// "I like pizza" -> "I pizza").
fn remove_filler_like(text: &str) -> String {
    let re = re_like();
    let mut result = String::with_capacity(text.len());
    let mut last_end = 0;

    for mat in re.find_iter(text) {
        let match_str = mat.as_str();
        let has_leading_comma = match_str.trim_start().starts_with(',')
            || text[..mat.start()].trim_end().ends_with(',');
        let has_trailing_comma = match_str.trim_end().ends_with(',')
            || text[mat.end()..].trim_start().starts_with(',');

        let at_sentence_start = {
            let before = text[..mat.start()].trim_end();
            before.is_empty()
                || before.ends_with('.')
                || before.ends_with('!')
                || before.ends_with('?')
        };

        let is_filler = (has_leading_comma && has_trailing_comma)
            || (at_sentence_start && has_trailing_comma);

        if is_filler {
            let before_text = &text[last_end..mat.start()];
            result.push_str(before_text);
            result.push(' ');
        } else {
            result.push_str(&text[last_end..mat.end()]);
        }
        last_end = mat.end();
    }
    result.push_str(&text[last_end..]);
    result
}

const YOU_KNOW_KEEP_FOLLOWING: &[&str] = &[
    "what", "who", "where", "when", "why", "how",
    "that", "if", "about", "anything", "something",
];

fn remove_filler_you_know(text: &str) -> String {
    let re = re_you_know();
    let mut result = String::with_capacity(text.len());
    let mut last_end = 0;

    for mat in re.find_iter(text) {
        let next = following_word(text, mat.end());
        let should_keep = match next {
            Some(ref word) => {
                let stem = strip_contraction(word);
                YOU_KNOW_KEEP_FOLLOWING.contains(&stem.as_str())
            }
            None => false,
        };

        if should_keep {
            result.push_str(&text[last_end..mat.end()]);
        } else {
            result.push_str(&text[last_end..mat.start()]);
            let match_str = mat.as_str();
            let has_leading_comma = match_str.contains(',')
                || text[..mat.start()].trim_end().ends_with(',');
            if has_leading_comma && !text[..mat.start()].trim_end().is_empty() {
                if !text[last_end..mat.start()].trim_end().ends_with(',') {
                    result.push(',');
                }
            }
            result.push(' ');
        }
        last_end = mat.end();
    }
    result.push_str(&text[last_end..]);
    result
}

const I_MEAN_KEEP_FOLLOWING: &[&str] = &[
    "it", "that", "this", "to", "the", "a", "an", "what",
];

fn remove_filler_i_mean(text: &str) -> String {
    let re = re_i_mean();
    let mut result = String::with_capacity(text.len());
    let mut last_end = 0;

    for mat in re.find_iter(text) {
        let match_text = mat.as_str();
        let has_comma = match_text.contains(',');
        let next = following_word(text, mat.end());

        let should_keep = if has_comma {
            false
        } else {
            match next {
                Some(ref word) => {
                    let stem = strip_contraction(word);
                    I_MEAN_KEEP_FOLLOWING.contains(&stem.as_str())
                }
                None => false,
            }
        };

        if should_keep {
            result.push_str(&text[last_end..mat.end()]);
        } else {
            result.push_str(&text[last_end..mat.start()]);
            result.push(' ');
        }
        last_end = mat.end();
    }
    result.push_str(&text[last_end..]);
    result
}

/// Remove "sort of" / "kind of" only in filler positions:
/// - Comma-wrapped: "it was, sort of, difficult" -> "it was difficult"
/// - Sentence start: "Sort of like a..." -> remove
/// Preserve when used as determiner: "What kind of car" -> kept
fn remove_filler_sort_kind_of(text: &str) -> String {
    let mut result = text.to_string();
    for re in &[re_sort_of(), re_kind_of()] {
        let input = result.clone();
        let mut output = String::with_capacity(input.len());
        let mut last_end = 0;

        for mat in re.find_iter(&input) {
            let match_str = mat.as_str();
            let has_comma = match_str.contains(',')
                || input[..mat.start()].trim_end().ends_with(',');
            let has_trailing_comma = match_str.trim_end().ends_with(',')
                || input[mat.end()..].trim_start().starts_with(',');
            let at_sentence_start = {
                let before = input[..mat.start()].trim_end();
                before.is_empty()
                    || before.ends_with('.')
                    || before.ends_with('!')
                    || before.ends_with('?')
            };

            let is_filler = has_comma || at_sentence_start;

            if is_filler {
                output.push_str(&input[last_end..mat.start()]);
                // Preserve structural comma when removing comma-wrapped filler
                if has_comma && has_trailing_comma && !at_sentence_start
                    && !input[..mat.start()].trim_end().is_empty()
                {
                    if !output.trim_end().ends_with(',') {
                        output.push(',');
                    }
                }
                output.push(' ');
            } else {
                output.push_str(&input[last_end..mat.end()]);
            }
            last_end = mat.end();
        }
        output.push_str(&input[last_end..]);
        result = output;
    }
    result
}

/// Remove "basically" only in filler positions:
/// - Sentence start: "Basically, we need to..." -> "We need to..."
/// - Comma-wrapped: "so, basically, it works" -> "so, it works"
/// Preserve mid-sentence: "The system is basically a cache" -> kept
fn remove_filler_basically(text: &str) -> String {
    let re = re_basically();
    let mut result = String::with_capacity(text.len());
    let mut last_end = 0;

    for mat in re.find_iter(text) {
        let match_str = mat.as_str();
        let has_comma = match_str.contains(',')
            || text[..mat.start()].trim_end().ends_with(',');
        let has_trailing_comma = match_str.trim_end().ends_with(',')
            || text[mat.end()..].trim_start().starts_with(',');
        let at_sentence_start = {
            let before = text[..mat.start()].trim_end();
            before.is_empty()
                || before.ends_with('.')
                || before.ends_with('!')
                || before.ends_with('?')
        };

        let is_filler = has_comma || at_sentence_start;

        if is_filler {
            result.push_str(&text[last_end..mat.start()]);
            // Preserve structural comma when removing comma-wrapped filler
            if has_comma && has_trailing_comma && !at_sentence_start
                && !text[..mat.start()].trim_end().is_empty()
            {
                if !result.trim_end().ends_with(',') {
                    result.push(',');
                }
            }
            result.push(' ');
        } else {
            result.push_str(&text[last_end..mat.end()]);
        }
        last_end = mat.end();
    }
    result.push_str(&text[last_end..]);
    result
}

// ---------------------------------------------------------------------------
// Post-Filler Cleanup Functions
// ---------------------------------------------------------------------------

/// Remove orphaned commas left by filler stripping.
fn clean_orphaned_commas(text: &str) -> String {
    let result = re_orphaned_commas().replace_all(text, ",");
    let result = result.trim_start_matches(|c: char| c == ',' || c.is_whitespace());
    result.to_string()
}

/// Collapse multiple whitespace characters to a single space.
fn collapse_whitespace(text: &str) -> String {
    re_multi_space().replace_all(text, " ").into_owned()
}

/// Capitalize the first letter of each sentence.
fn capitalize_sentences(text: &str) -> String {
    if text.is_empty() {
        return text.to_string();
    }

    let mut chars: Vec<char> = text.chars().collect();
    let mut capitalize_next = true;

    for i in 0..chars.len() {
        if capitalize_next && chars[i].is_alphabetic() {
            chars[i] = chars[i].to_uppercase().next().unwrap_or(chars[i]);
            capitalize_next = false;
        } else if chars[i] == '.' || chars[i] == '!' || chars[i] == '?' {
            capitalize_next = true;
        } else if chars[i].is_whitespace() {
            // keep capitalize_next as-is
        } else if capitalize_next && !chars[i].is_alphabetic() {
            // non-letter, non-whitespace after punctuation -- keep waiting
        } else {
            capitalize_next = false;
        }
    }

    chars.into_iter().collect()
}

/// Ensure text ends with a period if not already punctuated.
/// Only . ! ? are terminal punctuation (NOT : or ;).
fn ensure_trailing_period(text: &str) -> String {
    if text.is_empty() {
        return text.to_string();
    }
    let trimmed = text.trim_end();
    if trimmed.is_empty() {
        return String::new();
    }
    match trimmed.chars().last() {
        Some('.' | '!' | '?') => trimmed.to_string(),
        _ => format!("{}.", trimmed),
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Apply the full cleanup pipeline.
///
/// - If `filler_removal` is false, return text trimmed (passthrough).
/// - If `language` is not English (and not "auto"), return text trimmed (passthrough).
/// - Otherwise, run the full filler removal + formatting pipeline.
pub fn clean_transcription(raw: &str, filler_removal: bool, language: &str) -> String {
    let text = raw.trim().to_string();
    if text.is_empty() {
        return text;
    }

    if !filler_removal {
        return text;
    }

    // Language guard -- English patterns only for English
    if !should_apply_filler_removal(language) {
        return text;
    }

    // Pipeline (order matters -- context-sensitive BEFORE simple):
    let text = remove_filler_like(&text);            // Step 1
    let text = remove_filler_you_know(&text);        // Step 2
    let text = remove_filler_i_mean(&text);          // Step 3
    let text = remove_filler_sort_kind_of(&text);    // Step 4
    let text = remove_filler_basically(&text);       // Step 5
    let text = remove_simple_fillers(&text);         // Step 6
    let text = clean_orphaned_commas(&text);         // Step 7
    let text = collapse_whitespace(&text);           // Step 8
    let text = text.trim().to_string();              // Step 9
    let text = capitalize_sentences(&text);          // Step 10
    let text = ensure_trailing_period(&text);        // Step 11

    text
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: shorthand for English + filler_removal=true
    fn clean(s: &str) -> String {
        clean_transcription(s, true, "en")
    }

    // ================================================================
    // FILLER REMOVAL TESTS
    // ================================================================

    /// Test 1: Simple filler "Um" at sentence start is removed.
    #[test]
    fn test_simple_fillers_removed() {
        assert_eq!(clean("Um, I went to the store"), "I went to the store.");
    }

    /// Test 2: Mid-sentence "uh" surrounded by commas is removed.
    #[test]
    fn test_uh_removed() {
        assert_eq!(clean("I was, uh, thinking about it"), "I was thinking about it.");
    }

    /// Test 3: Multiple fillers in a single utterance are all removed.
    #[test]
    fn test_multiple_fillers() {
        let result = clean("Um, uh, so I was, like, going");
        assert_eq!(result, "So I was going.");
    }

    /// Test 4: "you know" as a filler (comma-wrapped, no keep-word following).
    #[test]
    fn test_you_know_filler() {
        assert_eq!(clean("It was, you know, really good"), "It was, really good.");
    }

    /// Test 5: "you know" as real content is preserved.
    #[test]
    fn test_you_know_real() {
        assert_eq!(clean("You know what happened"), "You know what happened.");
    }

    /// Test 6: "I mean" as filler (with comma) is removed.
    #[test]
    fn test_i_mean_filler() {
        assert_eq!(clean("I mean, it was fine"), "It was fine.");
    }

    /// Test 7: "I mean" as real content is preserved.
    #[test]
    fn test_i_mean_real() {
        assert_eq!(clean("I mean what I said"), "I mean what I said.");
    }

    /// Test 8: "like" as filler when comma-wrapped is removed.
    #[test]
    fn test_like_filler_comma() {
        assert_eq!(clean("It was, like, amazing"), "It was amazing.");
    }

    /// Test 9: "like" as a verb is preserved.
    #[test]
    fn test_like_real_verb() {
        assert_eq!(clean("I like pizza"), "I like pizza.");
    }

    /// Test 10: "like" as a preposition is preserved.
    #[test]
    fn test_like_real_preposition() {
        assert_eq!(
            clean("Things like shirts are nice"),
            "Things like shirts are nice."
        );
    }

    /// Test 11: "Like" at sentence start with comma is removed as filler.
    #[test]
    fn test_like_sentence_start() {
        assert_eq!(
            clean("Like, I don't even know"),
            "I don't even know."
        );
    }

    // ================================================================
    // TEXT CLEANUP TESTS
    // ================================================================

    /// Test 12: Double/triple spaces are collapsed to single space.
    #[test]
    fn test_double_spaces_collapsed() {
        assert_eq!(
            clean("I  went   to   the  store"),
            "I went to the store."
        );
    }

    /// Test 13: First letter of each sentence is capitalized.
    #[test]
    fn test_sentence_capitalization() {
        assert_eq!(
            clean("hello world. this is great"),
            "Hello world. This is great."
        );
    }

    /// Test 14: A trailing period is added when no terminal punctuation exists.
    #[test]
    fn test_trailing_period_added() {
        assert_eq!(clean("I went to the store"), "I went to the store.");
    }

    /// Test 15: An existing trailing period is not doubled.
    #[test]
    fn test_trailing_period_not_doubled() {
        assert_eq!(clean("I went to the store."), "I went to the store.");
    }

    /// Test 16: Existing question mark is preserved (no period added).
    #[test]
    fn test_existing_question_mark_preserved() {
        assert_eq!(clean("Did you go to the store?"), "Did you go to the store?");
    }

    /// Test 17: Existing exclamation mark is preserved (no period added).
    #[test]
    fn test_existing_exclamation_preserved() {
        assert_eq!(clean("That was amazing!"), "That was amazing!");
    }

    // ================================================================
    // EDGE CASES
    // ================================================================

    /// Test 18: Empty string input returns empty string.
    #[test]
    fn test_empty_input() {
        assert_eq!(clean(""), "");
    }

    /// Test 19: Input that is ALL fillers.
    #[test]
    fn test_all_filler_input() {
        let result = clean("Um, uh, like, you know");
        assert_eq!(result, "");
    }

    /// Test 20: Single word input gets a trailing period.
    #[test]
    fn test_single_word() {
        assert_eq!(clean("Hello"), "Hello.");
    }

    /// Test 21: Whitespace-only input is trimmed to empty.
    #[test]
    fn test_whitespace_only() {
        assert_eq!(clean("   "), "");
    }

    // ================================================================
    // LANGUAGE GUARD
    // ================================================================

    /// Test 22: Non-English text (German) passes through without modification.
    #[test]
    fn test_non_english_passthrough() {
        assert_eq!(
            clean_transcription("Er sagte dass er kommen will", true, "de"),
            "Er sagte dass er kommen will"
        );
    }

    /// Test 23: English text with the same structure has fillers removed.
    #[test]
    fn test_english_fillers_removed() {
        assert_eq!(
            clean_transcription("Er I think so", true, "en"),
            "I think so."
        );
    }

    /// Test 24: "auto" language applies English filler removal.
    #[test]
    fn test_auto_language_applies_cleanup() {
        assert_eq!(
            clean_transcription("Um I was thinking", true, "auto"),
            "I was thinking."
        );
    }

    // ================================================================
    // STRUCTURAL COMMA PRESERVATION
    // ================================================================

    /// Test 25: Structural comma preserved when removing filler between clauses.
    #[test]
    fn test_comma_preserved_well_um_ok() {
        assert_eq!(clean("Well, um, OK"), "Well, OK.");
    }

    // ================================================================
    // ADDITIONAL COVERAGE
    // ================================================================

    // --- "ah" is preserved (FIX H2) ---

    #[test]
    fn test_ah_preserved_as_interjection() {
        assert_eq!(clean("Ah I see"), "Ah I see.");
    }

    #[test]
    fn test_ah_with_comma_preserved() {
        assert_eq!(clean("Ah, that makes sense"), "Ah, that makes sense.");
    }

    // --- Additional "like" false-positive protection (FIX H1) ---

    #[test]
    fn test_like_as_simile_preserved() {
        assert_eq!(clean("It looks like rain"), "It looks like rain.");
    }

    #[test]
    fn test_like_people_like_you_preserved() {
        assert_eq!(clean("People like you are great"), "People like you are great.");
    }

    #[test]
    fn test_like_without_commas_preserved() {
        let result = clean("And like we should go");
        assert!(result.contains("like"), "Without commas, 'like' should be preserved");
    }

    // --- Additional "you know" tests ---

    #[test]
    fn test_you_know_where_preserved() {
        assert_eq!(clean("You know where he went"), "You know where he went.");
    }

    #[test]
    fn test_you_know_whats_contraction_preserved() {
        assert_eq!(clean("Do you know what's going on"), "Do you know what's going on.");
    }

    // --- Additional "I mean" tests ---

    #[test]
    fn test_i_mean_it_preserved() {
        assert_eq!(clean("I mean it"), "I mean it.");
    }

    #[test]
    fn test_i_mean_thats_contraction_preserved() {
        assert_eq!(clean("I mean that's important"), "I mean that's important.");
    }

    // --- "sort of" / "kind of" context-sensitive (FIX M4) ---

    #[test]
    fn test_kind_of_as_determiner_preserved() {
        assert_eq!(clean("What kind of car is that"), "What kind of car is that.");
    }

    #[test]
    fn test_sort_of_mid_sentence_preserved() {
        assert_eq!(clean("It was sort of difficult"), "It was sort of difficult.");
    }

    #[test]
    fn test_kind_of_comma_wrapped_removed() {
        assert_eq!(clean("It was, kind of, weird"), "It was, weird.");
    }

    #[test]
    fn test_sort_of_at_sentence_start_removed() {
        assert_eq!(clean("Sort of like that"), "Like that.");
    }

    // --- "basically" context-sensitive (FIX M5) ---

    #[test]
    fn test_basically_mid_sentence_preserved() {
        assert_eq!(clean("The system is basically a cache"), "The system is basically a cache.");
    }

    #[test]
    fn test_basically_at_sentence_start_removed() {
        assert_eq!(clean("Basically we need to go"), "We need to go.");
    }

    #[test]
    fn test_basically_comma_wrapped_removed() {
        assert_eq!(clean("So, basically, it works"), "So, it works.");
    }

    // --- Contraction stripping (FIX C2) ---

    #[test]
    fn test_strip_contraction_basics() {
        assert_eq!(strip_contraction("that's"), "that");
        assert_eq!(strip_contraction("don't"), "don");
        assert_eq!(strip_contraction("they're"), "they");
        assert_eq!(strip_contraction("we've"), "we");
        assert_eq!(strip_contraction("he'll"), "he");
        assert_eq!(strip_contraction("she'd"), "she");
        assert_eq!(strip_contraction("hello"), "hello");
    }

    #[test]
    fn test_strip_contraction_no_false_match() {
        assert_eq!(strip_contraction("anything"), "anything");
        assert_eq!(strip_contraction("also"), "also");
        assert_eq!(strip_contraction("together"), "together");
    }

    // --- Passthrough when disabled ---

    #[test]
    fn test_passthrough_when_disabled() {
        assert_eq!(
            clean_transcription("um uh like yeah", false, "en"),
            "um uh like yeah"
        );
    }

    #[test]
    fn test_passthrough_only_trims() {
        assert_eq!(
            clean_transcription("  hello  ", false, "en"),
            "hello"
        );
    }

    // --- Capitalization ---

    #[test]
    fn test_capitalizes_after_exclamation() {
        assert_eq!(clean("yes! that is great"), "Yes! That is great.");
    }

    #[test]
    fn test_already_uppercase_unchanged() {
        assert_eq!(clean("HELLO WORLD"), "HELLO WORLD.");
    }

    // --- Trailing period edge cases (FIX M3) ---

    #[test]
    fn test_colon_gets_period_added() {
        assert_eq!(clean("Item one: something"), "Item one: something.");
    }

    // --- Pipeline order validation (FIX M1) ---

    #[test]
    fn test_pipeline_order_context_not_corrupted() {
        let result = clean("I, um, like, was thinking");
        assert!(
            !result.to_lowercase().contains("like"),
            "Comma-wrapped 'like' should be removed even when adjacent to 'um'"
        );
    }

    // --- Integration: multiple fillers + full pipeline ---

    #[test]
    fn test_integration_full_pipeline() {
        let result = clean("um so I was you know thinking about the uh project");
        assert!(result.contains("So I was"), "Should start with capitalized 'So I was'");
        assert!(result.contains("the project"), "Should end with 'the project'");
        assert!(result.ends_with('.'), "Should end with period");
        assert!(!result.contains("  "), "No double spaces");
    }

    // --- Structural comma preservation across filler types ---

    #[test]
    fn test_comma_preserved_removing_you_know_between_clauses() {
        assert_eq!(clean("First, you know, second"), "First, second.");
    }
}
