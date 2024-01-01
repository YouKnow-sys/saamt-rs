//! Default Pak Names.

/// name of all SFX Pak files.
pub const SFX_DEFAULT_PAK_NAMES: [&str; 9] = [
    "FEET", "GENRL", "PAIN_A", "SCRIPT", "SPC_EA", "SPC_FA", "SPC_GA", "SPC_NA", "SPC_PA",
];

/// name of all STREAM Pak files.
// we keep the empty name here just for compatibility reasons, an empty name isn't valid
pub const STREAM_DEFAULT_PAK_NAMES: [&str; 17] = [
    "AA", "ADVERTS", "", "AMBIENCE", "BEATS", "CH", "CO", "CR", "CUTSCENE", "DS", "HC", "MH", "MR",
    "NJ", "RE", "RG", "TK",
];
