use super::*;

#[test]
fn the_list_is_enable_whole_and_in_order() {
    // ENABLE as published: 172,820 words, aa first and zyzzyvas last
    assert_eq!(&PACKED[..4], b"ENB1");
    let all = all();
    assert_eq!(count(), 172_820);
    assert_eq!(all.len(), 172_820);
    assert_eq!((all[0].as_str(), all[1].as_str()), ("AA", "AAH"));
    assert_eq!(all[all.len() - 1], "ZYZZYVAS");
    assert!(all.windows(2).all(|p| p[0] < p[1]), "sorted, no repeats");
}

#[test]
fn everyday_words_are_real() {
    for w in [
        "CAT", "CATS", "REEF", "REEFS", "CRAB", "CRABS", "TILE", "TILES", "ABBA",
    ] {
        assert!(is_real(w), "{w}");
    }
}

#[test]
fn enables_short_words_are_real_and_newer_ones_are_not() {
    // ENABLE has these two- and three-letter words...
    for w in ["AA", "XU", "XI", "OX", "JO", "QAT", "AAH"] {
        assert!(is_real(w), "{w}");
    }
    // ...and lacks these later tournament additions (CR-Q11: plain ENABLE)
    for w in ["QI", "ZA", "OK", "EW", "ZEN", "QIS", "ZAS"] {
        assert!(!is_real(w), "{w}");
    }
}

#[test]
fn made_up_words_are_not_real() {
    for w in ["CATX", "FUJ", "ZZZ", "ZZZS", "CRABBLE", "AAZ", "ZYZZYVASS"] {
        assert!(!is_real(w), "{w}");
    }
}

#[test]
fn the_first_and_last_words_and_a_block_edge_are_found() {
    // the list's ends and the words either side of the first block edge
    // (the 64th and 65th words, abasing and abatable)
    for w in [
        "AA", "ZYZZYVAS", "ZYZZYVA", "ABASING", "ABATABLE", "ABDUCT", "ABDUCTED",
    ] {
        assert!(is_real(w), "{w}");
    }
    // the longest word, 28 letters
    assert!(is_real("ETHYLENEDIAMINETETRAACETATES"));
}

#[test]
fn only_two_or_more_capitals_can_be_real() {
    assert!(!is_real(""));
    assert!(!is_real("A"));
    assert!(!is_real("cat"));
    assert!(!is_real("Cat"));
    assert!(!is_real("CA T"));
    assert!(!is_real("CAT1"));
    assert!(!is_real("ÉCU"));
    assert!(!is_real(&"A".repeat(40)));
}

#[test]
fn the_ais_vocabulary_is_a_bit_for_each_enable_word() {
    assert_eq!(&COMMON[..4], b"ENC1");
    assert_eq!(COMMON.len(), COMMON_BITS + 172_820usize.div_ceil(8));
    let set: usize = COMMON[COMMON_BITS..]
        .iter()
        .map(|b| b.count_ones() as usize)
        .sum();
    // 44,505 at level 40 (35 for short words), less SHORT_CUT's 23 (R20.7)
    assert_eq!(common_count(), 44_482);
    assert_eq!(set, 44_482);
}

#[test]
fn everyday_words_are_common_and_scrabble_oddities_are_only_real() {
    // TRIAGE F11: the AI keeps to words players know
    for w in [
        "CAT", "THE", "ZOO", "AT", "OX", "CRAB", "TILES", "JUMPED", "AARDVARK",
    ] {
        assert!(is_common(w), "{w}");
    }
    // and the short words a player would not know as words (R20.7)
    for w in [
        "AA", "XU", "QAT", "JO", "OE", "AAH", "ZYZZYVAS", "BESOM", "EM", "MU", "PI", "RE", "ID", "HES",
    ] {
        assert!(is_real(w) && !is_common(w), "{w}");
    }
    for w in ["", "A", "cat", "CATX", "ZZZ"] {
        assert!(!is_common(w), "{w:?}");
    }
}
