//! Generated deprecated ABI/source compatibility; do not edit.
//! Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10.
#![allow(deprecated, missing_docs)]
use std::ffi::{c_char, c_void};

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoAccess as CrabbyAccess;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum CrabbyService {
    CRABBY_SERVICE_INPUT = 0,

    CRABBY_SERVICE_TIME = 1,

    CRABBY_SERVICE_RANDOM = 2,

    CRABBY_SERVICE_GFX = 3,

    CRABBY_SERVICE_AUDIO = 4,

    CRABBY_SERVICE_ASSETS = 5,

    CRABBY_SERVICE_UI = 6,

    CRABBY_SERVICE_PERF = 7,

    CRABBY_SERVICE_CAMERA = 8,

    CRABBY_SERVICE_SPATIAL = 9,

    CRABBY_SERVICE_PHYSICS = 10,

    CRABBY_SERVICE_PHYSICS3D = 11,

    CRABBY_SERVICE_NET = 12,
}
impl From<crate::PonkoService> for CrabbyService {
    fn from(value: crate::PonkoService) -> Self {
        match value {
            crate::PonkoService::PONKO_SERVICE_INPUT => Self::CRABBY_SERVICE_INPUT,
            crate::PonkoService::PONKO_SERVICE_TIME => Self::CRABBY_SERVICE_TIME,
            crate::PonkoService::PONKO_SERVICE_RANDOM => Self::CRABBY_SERVICE_RANDOM,
            crate::PonkoService::PONKO_SERVICE_GFX => Self::CRABBY_SERVICE_GFX,
            crate::PonkoService::PONKO_SERVICE_AUDIO => Self::CRABBY_SERVICE_AUDIO,
            crate::PonkoService::PONKO_SERVICE_ASSETS => Self::CRABBY_SERVICE_ASSETS,
            crate::PonkoService::PONKO_SERVICE_UI => Self::CRABBY_SERVICE_UI,
            crate::PonkoService::PONKO_SERVICE_PERF => Self::CRABBY_SERVICE_PERF,
            crate::PonkoService::PONKO_SERVICE_CAMERA => Self::CRABBY_SERVICE_CAMERA,
            crate::PonkoService::PONKO_SERVICE_SPATIAL => Self::CRABBY_SERVICE_SPATIAL,
            crate::PonkoService::PONKO_SERVICE_PHYSICS => Self::CRABBY_SERVICE_PHYSICS,
            crate::PonkoService::PONKO_SERVICE_PHYSICS3D => Self::CRABBY_SERVICE_PHYSICS3D,
            crate::PonkoService::PONKO_SERVICE_NET => Self::CRABBY_SERVICE_NET,
        }
    }
}
impl From<CrabbyService> for crate::PonkoService {
    fn from(value: CrabbyService) -> Self {
        match value {
            CrabbyService::CRABBY_SERVICE_INPUT => Self::PONKO_SERVICE_INPUT,
            CrabbyService::CRABBY_SERVICE_TIME => Self::PONKO_SERVICE_TIME,
            CrabbyService::CRABBY_SERVICE_RANDOM => Self::PONKO_SERVICE_RANDOM,
            CrabbyService::CRABBY_SERVICE_GFX => Self::PONKO_SERVICE_GFX,
            CrabbyService::CRABBY_SERVICE_AUDIO => Self::PONKO_SERVICE_AUDIO,
            CrabbyService::CRABBY_SERVICE_ASSETS => Self::PONKO_SERVICE_ASSETS,
            CrabbyService::CRABBY_SERVICE_UI => Self::PONKO_SERVICE_UI,
            CrabbyService::CRABBY_SERVICE_PERF => Self::PONKO_SERVICE_PERF,
            CrabbyService::CRABBY_SERVICE_CAMERA => Self::PONKO_SERVICE_CAMERA,
            CrabbyService::CRABBY_SERVICE_SPATIAL => Self::PONKO_SERVICE_SPATIAL,
            CrabbyService::CRABBY_SERVICE_PHYSICS => Self::PONKO_SERVICE_PHYSICS,
            CrabbyService::CRABBY_SERVICE_PHYSICS3D => Self::PONKO_SERVICE_PHYSICS3D,
            CrabbyService::CRABBY_SERVICE_NET => Self::PONKO_SERVICE_NET,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyService>() == std::mem::size_of::<crate::PonkoService>());
    assert!(std::mem::align_of::<CrabbyService>() == std::mem::align_of::<crate::PonkoService>());
    assert!(CrabbyService::CRABBY_SERVICE_INPUT as u32 == crate::PonkoService::PONKO_SERVICE_INPUT as u32);
    assert!(CrabbyService::CRABBY_SERVICE_TIME as u32 == crate::PonkoService::PONKO_SERVICE_TIME as u32);
    assert!(CrabbyService::CRABBY_SERVICE_RANDOM as u32 == crate::PonkoService::PONKO_SERVICE_RANDOM as u32);
    assert!(CrabbyService::CRABBY_SERVICE_GFX as u32 == crate::PonkoService::PONKO_SERVICE_GFX as u32);
    assert!(CrabbyService::CRABBY_SERVICE_AUDIO as u32 == crate::PonkoService::PONKO_SERVICE_AUDIO as u32);
    assert!(CrabbyService::CRABBY_SERVICE_ASSETS as u32 == crate::PonkoService::PONKO_SERVICE_ASSETS as u32);
    assert!(CrabbyService::CRABBY_SERVICE_UI as u32 == crate::PonkoService::PONKO_SERVICE_UI as u32);
    assert!(CrabbyService::CRABBY_SERVICE_PERF as u32 == crate::PonkoService::PONKO_SERVICE_PERF as u32);
    assert!(CrabbyService::CRABBY_SERVICE_CAMERA as u32 == crate::PonkoService::PONKO_SERVICE_CAMERA as u32);
    assert!(CrabbyService::CRABBY_SERVICE_SPATIAL as u32 == crate::PonkoService::PONKO_SERVICE_SPATIAL as u32);
    assert!(CrabbyService::CRABBY_SERVICE_PHYSICS as u32 == crate::PonkoService::PONKO_SERVICE_PHYSICS as u32);
    assert!(CrabbyService::CRABBY_SERVICE_PHYSICS3D as u32 == crate::PonkoService::PONKO_SERVICE_PHYSICS3D as u32);
    assert!(CrabbyService::CRABBY_SERVICE_NET as u32 == crate::PonkoService::PONKO_SERVICE_NET as u32);
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoActionMap as CrabbyActionMap;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyService::CRABBY_SERVICE_ASSETS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyService::CRABBY_SERVICE_AUDIO;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyService::CRABBY_SERVICE_CAMERA;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyService::CRABBY_SERVICE_GFX;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyService::CRABBY_SERVICE_INPUT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyService::CRABBY_SERVICE_NET;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyService::CRABBY_SERVICE_PERF;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyService::CRABBY_SERVICE_PHYSICS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyService::CRABBY_SERVICE_PHYSICS3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyService::CRABBY_SERVICE_RANDOM;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyService::CRABBY_SERVICE_SPATIAL;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyService::CRABBY_SERVICE_TIME;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyService::CRABBY_SERVICE_UI;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum CrabbyActionSource {
    CRABBY_ACTION_KEY = 0,

    CRABBY_ACTION_MOUSE = 1,

    CRABBY_ACTION_GAMEPAD_BUTTON = 2,

    CRABBY_ACTION_KEYS = 3,

    CRABBY_ACTION_GAMEPAD_AXIS = 4,

    CRABBY_ACTION_KEYS2 = 5,

    CRABBY_ACTION_GAMEPAD_STICK = 6,
}
impl From<crate::PonkoActionSource> for CrabbyActionSource {
    fn from(value: crate::PonkoActionSource) -> Self {
        match value {
            crate::PonkoActionSource::PONKO_ACTION_KEY => Self::CRABBY_ACTION_KEY,
            crate::PonkoActionSource::PONKO_ACTION_MOUSE => Self::CRABBY_ACTION_MOUSE,
            crate::PonkoActionSource::PONKO_ACTION_GAMEPAD_BUTTON => Self::CRABBY_ACTION_GAMEPAD_BUTTON,
            crate::PonkoActionSource::PONKO_ACTION_KEYS => Self::CRABBY_ACTION_KEYS,
            crate::PonkoActionSource::PONKO_ACTION_GAMEPAD_AXIS => Self::CRABBY_ACTION_GAMEPAD_AXIS,
            crate::PonkoActionSource::PONKO_ACTION_KEYS2 => Self::CRABBY_ACTION_KEYS2,
            crate::PonkoActionSource::PONKO_ACTION_GAMEPAD_STICK => Self::CRABBY_ACTION_GAMEPAD_STICK,
        }
    }
}
impl From<CrabbyActionSource> for crate::PonkoActionSource {
    fn from(value: CrabbyActionSource) -> Self {
        match value {
            CrabbyActionSource::CRABBY_ACTION_KEY => Self::PONKO_ACTION_KEY,
            CrabbyActionSource::CRABBY_ACTION_MOUSE => Self::PONKO_ACTION_MOUSE,
            CrabbyActionSource::CRABBY_ACTION_GAMEPAD_BUTTON => Self::PONKO_ACTION_GAMEPAD_BUTTON,
            CrabbyActionSource::CRABBY_ACTION_KEYS => Self::PONKO_ACTION_KEYS,
            CrabbyActionSource::CRABBY_ACTION_GAMEPAD_AXIS => Self::PONKO_ACTION_GAMEPAD_AXIS,
            CrabbyActionSource::CRABBY_ACTION_KEYS2 => Self::PONKO_ACTION_KEYS2,
            CrabbyActionSource::CRABBY_ACTION_GAMEPAD_STICK => Self::PONKO_ACTION_GAMEPAD_STICK,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyActionSource>() == std::mem::size_of::<crate::PonkoActionSource>());
    assert!(std::mem::align_of::<CrabbyActionSource>() == std::mem::align_of::<crate::PonkoActionSource>());
    assert!(CrabbyActionSource::CRABBY_ACTION_KEY as u32 == crate::PonkoActionSource::PONKO_ACTION_KEY as u32);
    assert!(CrabbyActionSource::CRABBY_ACTION_MOUSE as u32 == crate::PonkoActionSource::PONKO_ACTION_MOUSE as u32);
    assert!(
        CrabbyActionSource::CRABBY_ACTION_GAMEPAD_BUTTON as u32
            == crate::PonkoActionSource::PONKO_ACTION_GAMEPAD_BUTTON as u32
    );
    assert!(CrabbyActionSource::CRABBY_ACTION_KEYS as u32 == crate::PonkoActionSource::PONKO_ACTION_KEYS as u32);
    assert!(
        CrabbyActionSource::CRABBY_ACTION_GAMEPAD_AXIS as u32
            == crate::PonkoActionSource::PONKO_ACTION_GAMEPAD_AXIS as u32
    );
    assert!(CrabbyActionSource::CRABBY_ACTION_KEYS2 as u32 == crate::PonkoActionSource::PONKO_ACTION_KEYS2 as u32);
    assert!(
        CrabbyActionSource::CRABBY_ACTION_GAMEPAD_STICK as u32
            == crate::PonkoActionSource::PONKO_ACTION_GAMEPAD_STICK as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoActionBinding as CrabbyActionBinding;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoAnimationGraph as CrabbyAnimationGraph;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoAnimationGraphBuilder as CrabbyAnimationGraphBuilder;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoMotion as CrabbyMotion;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyActionSource::CRABBY_ACTION_GAMEPAD_AXIS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyActionSource::CRABBY_ACTION_GAMEPAD_BUTTON;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyActionSource::CRABBY_ACTION_GAMEPAD_STICK;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyActionSource::CRABBY_ACTION_KEY;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyActionSource::CRABBY_ACTION_KEYS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyActionSource::CRABBY_ACTION_KEYS2;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyActionSource::CRABBY_ACTION_MOUSE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyAnimationCondition {
    CRABBY_CONDITION_GREATER = 0,

    CRABBY_CONDITION_LESS = 1,

    CRABBY_CONDITION_BOOL = 2,

    CRABBY_CONDITION_TRIGGER = 3,
}
impl From<crate::PonkoAnimationCondition> for CrabbyAnimationCondition {
    fn from(value: crate::PonkoAnimationCondition) -> Self {
        match value {
            crate::PonkoAnimationCondition::PONKO_CONDITION_GREATER => Self::CRABBY_CONDITION_GREATER,
            crate::PonkoAnimationCondition::PONKO_CONDITION_LESS => Self::CRABBY_CONDITION_LESS,
            crate::PonkoAnimationCondition::PONKO_CONDITION_BOOL => Self::CRABBY_CONDITION_BOOL,
            crate::PonkoAnimationCondition::PONKO_CONDITION_TRIGGER => Self::CRABBY_CONDITION_TRIGGER,
        }
    }
}
impl From<CrabbyAnimationCondition> for crate::PonkoAnimationCondition {
    fn from(value: CrabbyAnimationCondition) -> Self {
        match value {
            CrabbyAnimationCondition::CRABBY_CONDITION_GREATER => Self::PONKO_CONDITION_GREATER,
            CrabbyAnimationCondition::CRABBY_CONDITION_LESS => Self::PONKO_CONDITION_LESS,
            CrabbyAnimationCondition::CRABBY_CONDITION_BOOL => Self::PONKO_CONDITION_BOOL,
            CrabbyAnimationCondition::CRABBY_CONDITION_TRIGGER => Self::PONKO_CONDITION_TRIGGER,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyAnimationCondition>() == std::mem::size_of::<crate::PonkoAnimationCondition>());
    assert!(std::mem::align_of::<CrabbyAnimationCondition>() == std::mem::align_of::<crate::PonkoAnimationCondition>());
    assert!(
        CrabbyAnimationCondition::CRABBY_CONDITION_GREATER as u32
            == crate::PonkoAnimationCondition::PONKO_CONDITION_GREATER as u32
    );
    assert!(
        CrabbyAnimationCondition::CRABBY_CONDITION_LESS as u32
            == crate::PonkoAnimationCondition::PONKO_CONDITION_LESS as u32
    );
    assert!(
        CrabbyAnimationCondition::CRABBY_CONDITION_BOOL as u32
            == crate::PonkoAnimationCondition::PONKO_CONDITION_BOOL as u32
    );
    assert!(
        CrabbyAnimationCondition::CRABBY_CONDITION_TRIGGER as u32
            == crate::PonkoAnimationCondition::PONKO_CONDITION_TRIGGER as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAnimationCondition::CRABBY_CONDITION_BOOL;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAnimationCondition::CRABBY_CONDITION_GREATER;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAnimationCondition::CRABBY_CONDITION_LESS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAnimationCondition::CRABBY_CONDITION_TRIGGER;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyLayerBlend {
    CRABBY_LAYER_OVERRIDE,

    CRABBY_LAYER_ADDITIVE,
}
impl From<crate::PonkoLayerBlend> for CrabbyLayerBlend {
    fn from(value: crate::PonkoLayerBlend) -> Self {
        match value {
            crate::PonkoLayerBlend::PONKO_LAYER_OVERRIDE => Self::CRABBY_LAYER_OVERRIDE,
            crate::PonkoLayerBlend::PONKO_LAYER_ADDITIVE => Self::CRABBY_LAYER_ADDITIVE,
        }
    }
}
impl From<CrabbyLayerBlend> for crate::PonkoLayerBlend {
    fn from(value: CrabbyLayerBlend) -> Self {
        match value {
            CrabbyLayerBlend::CRABBY_LAYER_OVERRIDE => Self::PONKO_LAYER_OVERRIDE,
            CrabbyLayerBlend::CRABBY_LAYER_ADDITIVE => Self::PONKO_LAYER_ADDITIVE,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyLayerBlend>() == std::mem::size_of::<crate::PonkoLayerBlend>());
    assert!(std::mem::align_of::<CrabbyLayerBlend>() == std::mem::align_of::<crate::PonkoLayerBlend>());
    assert!(CrabbyLayerBlend::CRABBY_LAYER_OVERRIDE as u32 == crate::PonkoLayerBlend::PONKO_LAYER_OVERRIDE as u32);
    assert!(CrabbyLayerBlend::CRABBY_LAYER_ADDITIVE as u32 == crate::PonkoLayerBlend::PONKO_LAYER_ADDITIVE as u32);
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyLayerBlend::CRABBY_LAYER_ADDITIVE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyLayerBlend::CRABBY_LAYER_OVERRIDE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyRootMotion {
    CRABBY_ROOT_MOTION_OFF,

    CRABBY_ROOT_MOTION_EXTRACT,

    CRABBY_ROOT_MOTION_APPLY,
}
impl From<crate::PonkoRootMotion> for CrabbyRootMotion {
    fn from(value: crate::PonkoRootMotion) -> Self {
        match value {
            crate::PonkoRootMotion::PONKO_ROOT_MOTION_OFF => Self::CRABBY_ROOT_MOTION_OFF,
            crate::PonkoRootMotion::PONKO_ROOT_MOTION_EXTRACT => Self::CRABBY_ROOT_MOTION_EXTRACT,
            crate::PonkoRootMotion::PONKO_ROOT_MOTION_APPLY => Self::CRABBY_ROOT_MOTION_APPLY,
        }
    }
}
impl From<CrabbyRootMotion> for crate::PonkoRootMotion {
    fn from(value: CrabbyRootMotion) -> Self {
        match value {
            CrabbyRootMotion::CRABBY_ROOT_MOTION_OFF => Self::PONKO_ROOT_MOTION_OFF,
            CrabbyRootMotion::CRABBY_ROOT_MOTION_EXTRACT => Self::PONKO_ROOT_MOTION_EXTRACT,
            CrabbyRootMotion::CRABBY_ROOT_MOTION_APPLY => Self::PONKO_ROOT_MOTION_APPLY,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyRootMotion>() == std::mem::size_of::<crate::PonkoRootMotion>());
    assert!(std::mem::align_of::<CrabbyRootMotion>() == std::mem::align_of::<crate::PonkoRootMotion>());
    assert!(CrabbyRootMotion::CRABBY_ROOT_MOTION_OFF as u32 == crate::PonkoRootMotion::PONKO_ROOT_MOTION_OFF as u32);
    assert!(
        CrabbyRootMotion::CRABBY_ROOT_MOTION_EXTRACT as u32 == crate::PonkoRootMotion::PONKO_ROOT_MOTION_EXTRACT as u32
    );
    assert!(
        CrabbyRootMotion::CRABBY_ROOT_MOTION_APPLY as u32 == crate::PonkoRootMotion::PONKO_ROOT_MOTION_APPLY as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoAttached as CrabbyAttached;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoAudioBus as CrabbyAudioBus;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoAudioEffects as CrabbyAudioEffects;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyRootMotion::CRABBY_ROOT_MOTION_APPLY;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyRootMotion::CRABBY_ROOT_MOTION_EXTRACT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyRootMotion::CRABBY_ROOT_MOTION_OFF;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyAudioRolloff {
    CRABBY_ROLLOFF_LOGARITHMIC = 0,

    CRABBY_ROLLOFF_LINEAR = 1,
}
impl From<crate::PonkoAudioRolloff> for CrabbyAudioRolloff {
    fn from(value: crate::PonkoAudioRolloff) -> Self {
        match value {
            crate::PonkoAudioRolloff::PONKO_ROLLOFF_LOGARITHMIC => Self::CRABBY_ROLLOFF_LOGARITHMIC,
            crate::PonkoAudioRolloff::PONKO_ROLLOFF_LINEAR => Self::CRABBY_ROLLOFF_LINEAR,
        }
    }
}
impl From<CrabbyAudioRolloff> for crate::PonkoAudioRolloff {
    fn from(value: CrabbyAudioRolloff) -> Self {
        match value {
            CrabbyAudioRolloff::CRABBY_ROLLOFF_LOGARITHMIC => Self::PONKO_ROLLOFF_LOGARITHMIC,
            CrabbyAudioRolloff::CRABBY_ROLLOFF_LINEAR => Self::PONKO_ROLLOFF_LINEAR,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyAudioRolloff>() == std::mem::size_of::<crate::PonkoAudioRolloff>());
    assert!(std::mem::align_of::<CrabbyAudioRolloff>() == std::mem::align_of::<crate::PonkoAudioRolloff>());
    assert!(
        CrabbyAudioRolloff::CRABBY_ROLLOFF_LOGARITHMIC as u32
            == crate::PonkoAudioRolloff::PONKO_ROLLOFF_LOGARITHMIC as u32
    );
    assert!(CrabbyAudioRolloff::CRABBY_ROLLOFF_LINEAR as u32 == crate::PonkoAudioRolloff::PONKO_ROLLOFF_LINEAR as u32);
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAudioRolloff::CRABBY_ROLLOFF_LINEAR;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAudioRolloff::CRABBY_ROLLOFF_LOGARITHMIC;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyAudioSettings3D {
    pub sound: CrabbySoundParams,

    pub bus: CrabbyAudioBus,

    pub effects: CrabbyAudioEffects,

    pub min_distance: f32,

    pub max_distance: f32,

    pub rolloff: CrabbyAudioRolloff,

    pub doppler: f32,
}
impl From<crate::PonkoAudioSettings3D> for CrabbyAudioSettings3D {
    fn from(value: crate::PonkoAudioSettings3D) -> Self {
        Self {
            sound: value.sound,
            bus: value.bus,
            effects: value.effects,
            min_distance: value.min_distance,
            max_distance: value.max_distance,
            rolloff: value.rolloff.into(),
            doppler: value.doppler,
        }
    }
}
impl From<CrabbyAudioSettings3D> for crate::PonkoAudioSettings3D {
    fn from(value: CrabbyAudioSettings3D) -> Self {
        Self {
            sound: value.sound,
            bus: value.bus,
            effects: value.effects,
            min_distance: value.min_distance,
            max_distance: value.max_distance,
            rolloff: value.rolloff.into(),
            doppler: value.doppler,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyAudioSettings3D>() == std::mem::size_of::<crate::PonkoAudioSettings3D>());
    assert!(std::mem::align_of::<CrabbyAudioSettings3D>() == std::mem::align_of::<crate::PonkoAudioSettings3D>());
    assert!(
        std::mem::offset_of!(CrabbyAudioSettings3D, sound) == std::mem::offset_of!(crate::PonkoAudioSettings3D, sound)
    );
    assert!(std::mem::offset_of!(CrabbyAudioSettings3D, bus) == std::mem::offset_of!(crate::PonkoAudioSettings3D, bus));
    assert!(
        std::mem::offset_of!(CrabbyAudioSettings3D, effects)
            == std::mem::offset_of!(crate::PonkoAudioSettings3D, effects)
    );
    assert!(
        std::mem::offset_of!(CrabbyAudioSettings3D, min_distance)
            == std::mem::offset_of!(crate::PonkoAudioSettings3D, min_distance)
    );
    assert!(
        std::mem::offset_of!(CrabbyAudioSettings3D, max_distance)
            == std::mem::offset_of!(crate::PonkoAudioSettings3D, max_distance)
    );
    assert!(
        std::mem::offset_of!(CrabbyAudioSettings3D, rolloff)
            == std::mem::offset_of!(crate::PonkoAudioSettings3D, rolloff)
    );
    assert!(
        std::mem::offset_of!(CrabbyAudioSettings3D, doppler)
            == std::mem::offset_of!(crate::PonkoAudioSettings3D, doppler)
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoAudioListener3D as CrabbyAudioListener3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoCamera as CrabbyCamera;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoContactHook3D as CrabbyContactHook3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoContactModifier3D as CrabbyContactModifier3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoContactPair3D as CrabbyContactPair3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoCtx as CrabbyCtx;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoFilter as CrabbyFilter;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoFlowField as CrabbyFlowField;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoFlowView as CrabbyFlowView;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoGame as CrabbyGame;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoNavGrid as CrabbyNavGrid;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoQueryFilter2D as CrabbyQueryFilter2D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoQueryOptions as CrabbyQueryOptions;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoSystemFn as CrabbySystemFn;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyPhase {
    CRABBY_PHASE_INPUT,

    CRABBY_PHASE_BEFORE_PHYSICS,

    CRABBY_PHASE_PHYSICS,

    CRABBY_PHASE_AFTER_PHYSICS,

    CRABBY_PHASE_SIM_LAST,

    CRABBY_PHASE_FRAME,

    CRABBY_PHASE_ANIMATE,

    CRABBY_PHASE_LATE,

    CRABBY_PHASE_DRAW,
}
impl From<crate::PonkoPhase> for CrabbyPhase {
    fn from(value: crate::PonkoPhase) -> Self {
        match value {
            crate::PonkoPhase::PONKO_PHASE_INPUT => Self::CRABBY_PHASE_INPUT,
            crate::PonkoPhase::PONKO_PHASE_BEFORE_PHYSICS => Self::CRABBY_PHASE_BEFORE_PHYSICS,
            crate::PonkoPhase::PONKO_PHASE_PHYSICS => Self::CRABBY_PHASE_PHYSICS,
            crate::PonkoPhase::PONKO_PHASE_AFTER_PHYSICS => Self::CRABBY_PHASE_AFTER_PHYSICS,
            crate::PonkoPhase::PONKO_PHASE_SIM_LAST => Self::CRABBY_PHASE_SIM_LAST,
            crate::PonkoPhase::PONKO_PHASE_FRAME => Self::CRABBY_PHASE_FRAME,
            crate::PonkoPhase::PONKO_PHASE_ANIMATE => Self::CRABBY_PHASE_ANIMATE,
            crate::PonkoPhase::PONKO_PHASE_LATE => Self::CRABBY_PHASE_LATE,
            crate::PonkoPhase::PONKO_PHASE_DRAW => Self::CRABBY_PHASE_DRAW,
        }
    }
}
impl From<CrabbyPhase> for crate::PonkoPhase {
    fn from(value: CrabbyPhase) -> Self {
        match value {
            CrabbyPhase::CRABBY_PHASE_INPUT => Self::PONKO_PHASE_INPUT,
            CrabbyPhase::CRABBY_PHASE_BEFORE_PHYSICS => Self::PONKO_PHASE_BEFORE_PHYSICS,
            CrabbyPhase::CRABBY_PHASE_PHYSICS => Self::PONKO_PHASE_PHYSICS,
            CrabbyPhase::CRABBY_PHASE_AFTER_PHYSICS => Self::PONKO_PHASE_AFTER_PHYSICS,
            CrabbyPhase::CRABBY_PHASE_SIM_LAST => Self::PONKO_PHASE_SIM_LAST,
            CrabbyPhase::CRABBY_PHASE_FRAME => Self::PONKO_PHASE_FRAME,
            CrabbyPhase::CRABBY_PHASE_ANIMATE => Self::PONKO_PHASE_ANIMATE,
            CrabbyPhase::CRABBY_PHASE_LATE => Self::PONKO_PHASE_LATE,
            CrabbyPhase::CRABBY_PHASE_DRAW => Self::PONKO_PHASE_DRAW,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyPhase>() == std::mem::size_of::<crate::PonkoPhase>());
    assert!(std::mem::align_of::<CrabbyPhase>() == std::mem::align_of::<crate::PonkoPhase>());
    assert!(CrabbyPhase::CRABBY_PHASE_INPUT as u32 == crate::PonkoPhase::PONKO_PHASE_INPUT as u32);
    assert!(CrabbyPhase::CRABBY_PHASE_BEFORE_PHYSICS as u32 == crate::PonkoPhase::PONKO_PHASE_BEFORE_PHYSICS as u32);
    assert!(CrabbyPhase::CRABBY_PHASE_PHYSICS as u32 == crate::PonkoPhase::PONKO_PHASE_PHYSICS as u32);
    assert!(CrabbyPhase::CRABBY_PHASE_AFTER_PHYSICS as u32 == crate::PonkoPhase::PONKO_PHASE_AFTER_PHYSICS as u32);
    assert!(CrabbyPhase::CRABBY_PHASE_SIM_LAST as u32 == crate::PonkoPhase::PONKO_PHASE_SIM_LAST as u32);
    assert!(CrabbyPhase::CRABBY_PHASE_FRAME as u32 == crate::PonkoPhase::PONKO_PHASE_FRAME as u32);
    assert!(CrabbyPhase::CRABBY_PHASE_ANIMATE as u32 == crate::PonkoPhase::PONKO_PHASE_ANIMATE as u32);
    assert!(CrabbyPhase::CRABBY_PHASE_LATE as u32 == crate::PonkoPhase::PONKO_PHASE_LATE as u32);
    assert!(CrabbyPhase::CRABBY_PHASE_DRAW as u32 == crate::PonkoPhase::PONKO_PHASE_DRAW as u32);
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoPluginBuildFn as CrabbyPluginBuildFn;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyPhase::CRABBY_PHASE_AFTER_PHYSICS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyPhase::CRABBY_PHASE_ANIMATE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyPhase::CRABBY_PHASE_BEFORE_PHYSICS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyPhase::CRABBY_PHASE_DRAW;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyPhase::CRABBY_PHASE_FRAME;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyPhase::CRABBY_PHASE_INPUT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyPhase::CRABBY_PHASE_LATE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyPhase::CRABBY_PHASE_PHYSICS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyPhase::CRABBY_PHASE_SIM_LAST;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyGpuTier {
    CRABBY_GPU_TIER_WEBGL2 = 0,

    CRABBY_GPU_TIER_BASELINE = 1,

    CRABBY_GPU_TIER_DESKTOP = 2,
}
impl From<crate::PonkoGpuTier> for CrabbyGpuTier {
    fn from(value: crate::PonkoGpuTier) -> Self {
        match value {
            crate::PonkoGpuTier::PONKO_GPU_TIER_WEBGL2 => Self::CRABBY_GPU_TIER_WEBGL2,
            crate::PonkoGpuTier::PONKO_GPU_TIER_BASELINE => Self::CRABBY_GPU_TIER_BASELINE,
            crate::PonkoGpuTier::PONKO_GPU_TIER_DESKTOP => Self::CRABBY_GPU_TIER_DESKTOP,
        }
    }
}
impl From<CrabbyGpuTier> for crate::PonkoGpuTier {
    fn from(value: CrabbyGpuTier) -> Self {
        match value {
            CrabbyGpuTier::CRABBY_GPU_TIER_WEBGL2 => Self::PONKO_GPU_TIER_WEBGL2,
            CrabbyGpuTier::CRABBY_GPU_TIER_BASELINE => Self::PONKO_GPU_TIER_BASELINE,
            CrabbyGpuTier::CRABBY_GPU_TIER_DESKTOP => Self::PONKO_GPU_TIER_DESKTOP,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyGpuTier>() == std::mem::size_of::<crate::PonkoGpuTier>());
    assert!(std::mem::align_of::<CrabbyGpuTier>() == std::mem::align_of::<crate::PonkoGpuTier>());
    assert!(CrabbyGpuTier::CRABBY_GPU_TIER_WEBGL2 as u32 == crate::PonkoGpuTier::PONKO_GPU_TIER_WEBGL2 as u32);
    assert!(CrabbyGpuTier::CRABBY_GPU_TIER_BASELINE as u32 == crate::PonkoGpuTier::PONKO_GPU_TIER_BASELINE as u32);
    assert!(CrabbyGpuTier::CRABBY_GPU_TIER_DESKTOP as u32 == crate::PonkoGpuTier::PONKO_GPU_TIER_DESKTOP as u32);
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGpuTier::CRABBY_GPU_TIER_BASELINE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGpuTier::CRABBY_GPU_TIER_DESKTOP;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGpuTier::CRABBY_GPU_TIER_WEBGL2;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyGpuFeature {
    CRABBY_GPU_FEATURE_TIMESTAMPS = 0,

    CRABBY_GPU_FEATURE_COMPUTE = 1,

    CRABBY_GPU_FEATURE_STORAGE_BUFFERS = 2,

    CRABBY_GPU_FEATURE_COMPUTE_CULLING = 3,

    CRABBY_GPU_FEATURE_GPU_SKINNING = 4,

    CRABBY_GPU_FEATURE_OCCLUSION_CULLING = 5,

    CRABBY_GPU_FEATURE_TEMPORAL_ANTIALIASING = 6,

    CRABBY_GPU_FEATURE_AMBIENT_OCCLUSION = 7,

    CRABBY_GPU_FEATURE_CONTACT_SHADOWS = 8,

    CRABBY_GPU_FEATURE_SCREEN_SPACE_REFLECTIONS = 9,

    CRABBY_GPU_FEATURE_GLOBAL_ILLUMINATION = 10,

    CRABBY_GPU_FEATURE_DECALS = 11,

    CRABBY_GPU_FEATURE_MULTISAMPLING = 12,
}
impl From<crate::PonkoGpuFeature> for CrabbyGpuFeature {
    fn from(value: crate::PonkoGpuFeature) -> Self {
        match value {
            crate::PonkoGpuFeature::PONKO_GPU_FEATURE_TIMESTAMPS => Self::CRABBY_GPU_FEATURE_TIMESTAMPS,
            crate::PonkoGpuFeature::PONKO_GPU_FEATURE_COMPUTE => Self::CRABBY_GPU_FEATURE_COMPUTE,
            crate::PonkoGpuFeature::PONKO_GPU_FEATURE_STORAGE_BUFFERS => Self::CRABBY_GPU_FEATURE_STORAGE_BUFFERS,
            crate::PonkoGpuFeature::PONKO_GPU_FEATURE_COMPUTE_CULLING => Self::CRABBY_GPU_FEATURE_COMPUTE_CULLING,
            crate::PonkoGpuFeature::PONKO_GPU_FEATURE_GPU_SKINNING => Self::CRABBY_GPU_FEATURE_GPU_SKINNING,
            crate::PonkoGpuFeature::PONKO_GPU_FEATURE_OCCLUSION_CULLING => Self::CRABBY_GPU_FEATURE_OCCLUSION_CULLING,
            crate::PonkoGpuFeature::PONKO_GPU_FEATURE_TEMPORAL_ANTIALIASING => {
                Self::CRABBY_GPU_FEATURE_TEMPORAL_ANTIALIASING
            }
            crate::PonkoGpuFeature::PONKO_GPU_FEATURE_AMBIENT_OCCLUSION => Self::CRABBY_GPU_FEATURE_AMBIENT_OCCLUSION,
            crate::PonkoGpuFeature::PONKO_GPU_FEATURE_CONTACT_SHADOWS => Self::CRABBY_GPU_FEATURE_CONTACT_SHADOWS,
            crate::PonkoGpuFeature::PONKO_GPU_FEATURE_SCREEN_SPACE_REFLECTIONS => {
                Self::CRABBY_GPU_FEATURE_SCREEN_SPACE_REFLECTIONS
            }
            crate::PonkoGpuFeature::PONKO_GPU_FEATURE_GLOBAL_ILLUMINATION => {
                Self::CRABBY_GPU_FEATURE_GLOBAL_ILLUMINATION
            }
            crate::PonkoGpuFeature::PONKO_GPU_FEATURE_DECALS => Self::CRABBY_GPU_FEATURE_DECALS,
            crate::PonkoGpuFeature::PONKO_GPU_FEATURE_MULTISAMPLING => Self::CRABBY_GPU_FEATURE_MULTISAMPLING,
        }
    }
}
impl From<CrabbyGpuFeature> for crate::PonkoGpuFeature {
    fn from(value: CrabbyGpuFeature) -> Self {
        match value {
            CrabbyGpuFeature::CRABBY_GPU_FEATURE_TIMESTAMPS => Self::PONKO_GPU_FEATURE_TIMESTAMPS,
            CrabbyGpuFeature::CRABBY_GPU_FEATURE_COMPUTE => Self::PONKO_GPU_FEATURE_COMPUTE,
            CrabbyGpuFeature::CRABBY_GPU_FEATURE_STORAGE_BUFFERS => Self::PONKO_GPU_FEATURE_STORAGE_BUFFERS,
            CrabbyGpuFeature::CRABBY_GPU_FEATURE_COMPUTE_CULLING => Self::PONKO_GPU_FEATURE_COMPUTE_CULLING,
            CrabbyGpuFeature::CRABBY_GPU_FEATURE_GPU_SKINNING => Self::PONKO_GPU_FEATURE_GPU_SKINNING,
            CrabbyGpuFeature::CRABBY_GPU_FEATURE_OCCLUSION_CULLING => Self::PONKO_GPU_FEATURE_OCCLUSION_CULLING,
            CrabbyGpuFeature::CRABBY_GPU_FEATURE_TEMPORAL_ANTIALIASING => Self::PONKO_GPU_FEATURE_TEMPORAL_ANTIALIASING,
            CrabbyGpuFeature::CRABBY_GPU_FEATURE_AMBIENT_OCCLUSION => Self::PONKO_GPU_FEATURE_AMBIENT_OCCLUSION,
            CrabbyGpuFeature::CRABBY_GPU_FEATURE_CONTACT_SHADOWS => Self::PONKO_GPU_FEATURE_CONTACT_SHADOWS,
            CrabbyGpuFeature::CRABBY_GPU_FEATURE_SCREEN_SPACE_REFLECTIONS => {
                Self::PONKO_GPU_FEATURE_SCREEN_SPACE_REFLECTIONS
            }
            CrabbyGpuFeature::CRABBY_GPU_FEATURE_GLOBAL_ILLUMINATION => Self::PONKO_GPU_FEATURE_GLOBAL_ILLUMINATION,
            CrabbyGpuFeature::CRABBY_GPU_FEATURE_DECALS => Self::PONKO_GPU_FEATURE_DECALS,
            CrabbyGpuFeature::CRABBY_GPU_FEATURE_MULTISAMPLING => Self::PONKO_GPU_FEATURE_MULTISAMPLING,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyGpuFeature>() == std::mem::size_of::<crate::PonkoGpuFeature>());
    assert!(std::mem::align_of::<CrabbyGpuFeature>() == std::mem::align_of::<crate::PonkoGpuFeature>());
    assert!(
        CrabbyGpuFeature::CRABBY_GPU_FEATURE_TIMESTAMPS as u32
            == crate::PonkoGpuFeature::PONKO_GPU_FEATURE_TIMESTAMPS as u32
    );
    assert!(
        CrabbyGpuFeature::CRABBY_GPU_FEATURE_COMPUTE as u32 == crate::PonkoGpuFeature::PONKO_GPU_FEATURE_COMPUTE as u32
    );
    assert!(
        CrabbyGpuFeature::CRABBY_GPU_FEATURE_STORAGE_BUFFERS as u32
            == crate::PonkoGpuFeature::PONKO_GPU_FEATURE_STORAGE_BUFFERS as u32
    );
    assert!(
        CrabbyGpuFeature::CRABBY_GPU_FEATURE_COMPUTE_CULLING as u32
            == crate::PonkoGpuFeature::PONKO_GPU_FEATURE_COMPUTE_CULLING as u32
    );
    assert!(
        CrabbyGpuFeature::CRABBY_GPU_FEATURE_GPU_SKINNING as u32
            == crate::PonkoGpuFeature::PONKO_GPU_FEATURE_GPU_SKINNING as u32
    );
    assert!(
        CrabbyGpuFeature::CRABBY_GPU_FEATURE_OCCLUSION_CULLING as u32
            == crate::PonkoGpuFeature::PONKO_GPU_FEATURE_OCCLUSION_CULLING as u32
    );
    assert!(
        CrabbyGpuFeature::CRABBY_GPU_FEATURE_TEMPORAL_ANTIALIASING as u32
            == crate::PonkoGpuFeature::PONKO_GPU_FEATURE_TEMPORAL_ANTIALIASING as u32
    );
    assert!(
        CrabbyGpuFeature::CRABBY_GPU_FEATURE_AMBIENT_OCCLUSION as u32
            == crate::PonkoGpuFeature::PONKO_GPU_FEATURE_AMBIENT_OCCLUSION as u32
    );
    assert!(
        CrabbyGpuFeature::CRABBY_GPU_FEATURE_CONTACT_SHADOWS as u32
            == crate::PonkoGpuFeature::PONKO_GPU_FEATURE_CONTACT_SHADOWS as u32
    );
    assert!(
        CrabbyGpuFeature::CRABBY_GPU_FEATURE_SCREEN_SPACE_REFLECTIONS as u32
            == crate::PonkoGpuFeature::PONKO_GPU_FEATURE_SCREEN_SPACE_REFLECTIONS as u32
    );
    assert!(
        CrabbyGpuFeature::CRABBY_GPU_FEATURE_GLOBAL_ILLUMINATION as u32
            == crate::PonkoGpuFeature::PONKO_GPU_FEATURE_GLOBAL_ILLUMINATION as u32
    );
    assert!(
        CrabbyGpuFeature::CRABBY_GPU_FEATURE_DECALS as u32 == crate::PonkoGpuFeature::PONKO_GPU_FEATURE_DECALS as u32
    );
    assert!(
        CrabbyGpuFeature::CRABBY_GPU_FEATURE_MULTISAMPLING as u32
            == crate::PonkoGpuFeature::PONKO_GPU_FEATURE_MULTISAMPLING as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoGrid as CrabbyGrid;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoGridView as CrabbyGridView;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGpuFeature::CRABBY_GPU_FEATURE_AMBIENT_OCCLUSION;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGpuFeature::CRABBY_GPU_FEATURE_COMPUTE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGpuFeature::CRABBY_GPU_FEATURE_COMPUTE_CULLING;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGpuFeature::CRABBY_GPU_FEATURE_CONTACT_SHADOWS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGpuFeature::CRABBY_GPU_FEATURE_DECALS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGpuFeature::CRABBY_GPU_FEATURE_GLOBAL_ILLUMINATION;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGpuFeature::CRABBY_GPU_FEATURE_GPU_SKINNING;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGpuFeature::CRABBY_GPU_FEATURE_MULTISAMPLING;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGpuFeature::CRABBY_GPU_FEATURE_OCCLUSION_CULLING;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGpuFeature::CRABBY_GPU_FEATURE_SCREEN_SPACE_REFLECTIONS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGpuFeature::CRABBY_GPU_FEATURE_STORAGE_BUFFERS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGpuFeature::CRABBY_GPU_FEATURE_TEMPORAL_ANTIALIASING;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGpuFeature::CRABBY_GPU_FEATURE_TIMESTAMPS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyHandleKind {
    CRABBY_HANDLE_TEXTURE = 0,

    CRABBY_HANDLE_SOUND = 1,

    CRABBY_HANDLE_FONT = 2,

    CRABBY_HANDLE_MESH = 3,

    CRABBY_HANDLE_MATERIAL = 4,

    CRABBY_HANDLE_SHADER = 5,

    CRABBY_HANDLE_COMPUTE = 6,

    CRABBY_HANDLE_EFFECT = 7,

    CRABBY_HANDLE_GPU_BUFFER = 8,

    CRABBY_HANDLE_INSTANCE_BATCH = 9,

    CRABBY_HANDLE_TEXTURE_ARRAY = 10,
}
impl From<crate::PonkoHandleKind> for CrabbyHandleKind {
    fn from(value: crate::PonkoHandleKind) -> Self {
        match value {
            crate::PonkoHandleKind::PONKO_HANDLE_TEXTURE => Self::CRABBY_HANDLE_TEXTURE,
            crate::PonkoHandleKind::PONKO_HANDLE_SOUND => Self::CRABBY_HANDLE_SOUND,
            crate::PonkoHandleKind::PONKO_HANDLE_FONT => Self::CRABBY_HANDLE_FONT,
            crate::PonkoHandleKind::PONKO_HANDLE_MESH => Self::CRABBY_HANDLE_MESH,
            crate::PonkoHandleKind::PONKO_HANDLE_MATERIAL => Self::CRABBY_HANDLE_MATERIAL,
            crate::PonkoHandleKind::PONKO_HANDLE_SHADER => Self::CRABBY_HANDLE_SHADER,
            crate::PonkoHandleKind::PONKO_HANDLE_COMPUTE => Self::CRABBY_HANDLE_COMPUTE,
            crate::PonkoHandleKind::PONKO_HANDLE_EFFECT => Self::CRABBY_HANDLE_EFFECT,
            crate::PonkoHandleKind::PONKO_HANDLE_GPU_BUFFER => Self::CRABBY_HANDLE_GPU_BUFFER,
            crate::PonkoHandleKind::PONKO_HANDLE_INSTANCE_BATCH => Self::CRABBY_HANDLE_INSTANCE_BATCH,
            crate::PonkoHandleKind::PONKO_HANDLE_TEXTURE_ARRAY => Self::CRABBY_HANDLE_TEXTURE_ARRAY,
        }
    }
}
impl From<CrabbyHandleKind> for crate::PonkoHandleKind {
    fn from(value: CrabbyHandleKind) -> Self {
        match value {
            CrabbyHandleKind::CRABBY_HANDLE_TEXTURE => Self::PONKO_HANDLE_TEXTURE,
            CrabbyHandleKind::CRABBY_HANDLE_SOUND => Self::PONKO_HANDLE_SOUND,
            CrabbyHandleKind::CRABBY_HANDLE_FONT => Self::PONKO_HANDLE_FONT,
            CrabbyHandleKind::CRABBY_HANDLE_MESH => Self::PONKO_HANDLE_MESH,
            CrabbyHandleKind::CRABBY_HANDLE_MATERIAL => Self::PONKO_HANDLE_MATERIAL,
            CrabbyHandleKind::CRABBY_HANDLE_SHADER => Self::PONKO_HANDLE_SHADER,
            CrabbyHandleKind::CRABBY_HANDLE_COMPUTE => Self::PONKO_HANDLE_COMPUTE,
            CrabbyHandleKind::CRABBY_HANDLE_EFFECT => Self::PONKO_HANDLE_EFFECT,
            CrabbyHandleKind::CRABBY_HANDLE_GPU_BUFFER => Self::PONKO_HANDLE_GPU_BUFFER,
            CrabbyHandleKind::CRABBY_HANDLE_INSTANCE_BATCH => Self::PONKO_HANDLE_INSTANCE_BATCH,
            CrabbyHandleKind::CRABBY_HANDLE_TEXTURE_ARRAY => Self::PONKO_HANDLE_TEXTURE_ARRAY,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyHandleKind>() == std::mem::size_of::<crate::PonkoHandleKind>());
    assert!(std::mem::align_of::<CrabbyHandleKind>() == std::mem::align_of::<crate::PonkoHandleKind>());
    assert!(CrabbyHandleKind::CRABBY_HANDLE_TEXTURE as u32 == crate::PonkoHandleKind::PONKO_HANDLE_TEXTURE as u32);
    assert!(CrabbyHandleKind::CRABBY_HANDLE_SOUND as u32 == crate::PonkoHandleKind::PONKO_HANDLE_SOUND as u32);
    assert!(CrabbyHandleKind::CRABBY_HANDLE_FONT as u32 == crate::PonkoHandleKind::PONKO_HANDLE_FONT as u32);
    assert!(CrabbyHandleKind::CRABBY_HANDLE_MESH as u32 == crate::PonkoHandleKind::PONKO_HANDLE_MESH as u32);
    assert!(CrabbyHandleKind::CRABBY_HANDLE_MATERIAL as u32 == crate::PonkoHandleKind::PONKO_HANDLE_MATERIAL as u32);
    assert!(CrabbyHandleKind::CRABBY_HANDLE_SHADER as u32 == crate::PonkoHandleKind::PONKO_HANDLE_SHADER as u32);
    assert!(CrabbyHandleKind::CRABBY_HANDLE_COMPUTE as u32 == crate::PonkoHandleKind::PONKO_HANDLE_COMPUTE as u32);
    assert!(CrabbyHandleKind::CRABBY_HANDLE_EFFECT as u32 == crate::PonkoHandleKind::PONKO_HANDLE_EFFECT as u32);
    assert!(
        CrabbyHandleKind::CRABBY_HANDLE_GPU_BUFFER as u32 == crate::PonkoHandleKind::PONKO_HANDLE_GPU_BUFFER as u32
    );
    assert!(
        CrabbyHandleKind::CRABBY_HANDLE_INSTANCE_BATCH as u32
            == crate::PonkoHandleKind::PONKO_HANDLE_INSTANCE_BATCH as u32
    );
    assert!(
        CrabbyHandleKind::CRABBY_HANDLE_TEXTURE_ARRAY as u32
            == crate::PonkoHandleKind::PONKO_HANDLE_TEXTURE_ARRAY as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoLiveCounts as CrabbyLiveCounts;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyHandleKind::CRABBY_HANDLE_COMPUTE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyHandleKind::CRABBY_HANDLE_EFFECT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyHandleKind::CRABBY_HANDLE_FONT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyHandleKind::CRABBY_HANDLE_GPU_BUFFER;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyHandleKind::CRABBY_HANDLE_INSTANCE_BATCH;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyHandleKind::CRABBY_HANDLE_MATERIAL;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyHandleKind::CRABBY_HANDLE_MESH;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyHandleKind::CRABBY_HANDLE_SHADER;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyHandleKind::CRABBY_HANDLE_SOUND;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyHandleKind::CRABBY_HANDLE_TEXTURE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyHandleKind::CRABBY_HANDLE_TEXTURE_ARRAY;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyAssetClass {
    CRABBY_ASSET_CLASS_TEXTURE = 0,

    CRABBY_ASSET_CLASS_RENDER_TARGET = 1,

    CRABBY_ASSET_CLASS_TEXTURE_ARRAY = 2,

    CRABBY_ASSET_CLASS_MATERIAL = 3,

    CRABBY_ASSET_CLASS_SHADER = 4,

    CRABBY_ASSET_CLASS_MESH = 5,

    CRABBY_ASSET_CLASS_GPU_BUFFER = 6,

    CRABBY_ASSET_CLASS_EFFECT = 7,

    CRABBY_ASSET_CLASS_COMPUTE = 8,

    CRABBY_ASSET_CLASS_SOUND = 9,
}
impl From<crate::PonkoAssetClass> for CrabbyAssetClass {
    fn from(value: crate::PonkoAssetClass) -> Self {
        match value {
            crate::PonkoAssetClass::PONKO_ASSET_CLASS_TEXTURE => Self::CRABBY_ASSET_CLASS_TEXTURE,
            crate::PonkoAssetClass::PONKO_ASSET_CLASS_RENDER_TARGET => Self::CRABBY_ASSET_CLASS_RENDER_TARGET,
            crate::PonkoAssetClass::PONKO_ASSET_CLASS_TEXTURE_ARRAY => Self::CRABBY_ASSET_CLASS_TEXTURE_ARRAY,
            crate::PonkoAssetClass::PONKO_ASSET_CLASS_MATERIAL => Self::CRABBY_ASSET_CLASS_MATERIAL,
            crate::PonkoAssetClass::PONKO_ASSET_CLASS_SHADER => Self::CRABBY_ASSET_CLASS_SHADER,
            crate::PonkoAssetClass::PONKO_ASSET_CLASS_MESH => Self::CRABBY_ASSET_CLASS_MESH,
            crate::PonkoAssetClass::PONKO_ASSET_CLASS_GPU_BUFFER => Self::CRABBY_ASSET_CLASS_GPU_BUFFER,
            crate::PonkoAssetClass::PONKO_ASSET_CLASS_EFFECT => Self::CRABBY_ASSET_CLASS_EFFECT,
            crate::PonkoAssetClass::PONKO_ASSET_CLASS_COMPUTE => Self::CRABBY_ASSET_CLASS_COMPUTE,
            crate::PonkoAssetClass::PONKO_ASSET_CLASS_SOUND => Self::CRABBY_ASSET_CLASS_SOUND,
        }
    }
}
impl From<CrabbyAssetClass> for crate::PonkoAssetClass {
    fn from(value: CrabbyAssetClass) -> Self {
        match value {
            CrabbyAssetClass::CRABBY_ASSET_CLASS_TEXTURE => Self::PONKO_ASSET_CLASS_TEXTURE,
            CrabbyAssetClass::CRABBY_ASSET_CLASS_RENDER_TARGET => Self::PONKO_ASSET_CLASS_RENDER_TARGET,
            CrabbyAssetClass::CRABBY_ASSET_CLASS_TEXTURE_ARRAY => Self::PONKO_ASSET_CLASS_TEXTURE_ARRAY,
            CrabbyAssetClass::CRABBY_ASSET_CLASS_MATERIAL => Self::PONKO_ASSET_CLASS_MATERIAL,
            CrabbyAssetClass::CRABBY_ASSET_CLASS_SHADER => Self::PONKO_ASSET_CLASS_SHADER,
            CrabbyAssetClass::CRABBY_ASSET_CLASS_MESH => Self::PONKO_ASSET_CLASS_MESH,
            CrabbyAssetClass::CRABBY_ASSET_CLASS_GPU_BUFFER => Self::PONKO_ASSET_CLASS_GPU_BUFFER,
            CrabbyAssetClass::CRABBY_ASSET_CLASS_EFFECT => Self::PONKO_ASSET_CLASS_EFFECT,
            CrabbyAssetClass::CRABBY_ASSET_CLASS_COMPUTE => Self::PONKO_ASSET_CLASS_COMPUTE,
            CrabbyAssetClass::CRABBY_ASSET_CLASS_SOUND => Self::PONKO_ASSET_CLASS_SOUND,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyAssetClass>() == std::mem::size_of::<crate::PonkoAssetClass>());
    assert!(std::mem::align_of::<CrabbyAssetClass>() == std::mem::align_of::<crate::PonkoAssetClass>());
    assert!(
        CrabbyAssetClass::CRABBY_ASSET_CLASS_TEXTURE as u32 == crate::PonkoAssetClass::PONKO_ASSET_CLASS_TEXTURE as u32
    );
    assert!(
        CrabbyAssetClass::CRABBY_ASSET_CLASS_RENDER_TARGET as u32
            == crate::PonkoAssetClass::PONKO_ASSET_CLASS_RENDER_TARGET as u32
    );
    assert!(
        CrabbyAssetClass::CRABBY_ASSET_CLASS_TEXTURE_ARRAY as u32
            == crate::PonkoAssetClass::PONKO_ASSET_CLASS_TEXTURE_ARRAY as u32
    );
    assert!(
        CrabbyAssetClass::CRABBY_ASSET_CLASS_MATERIAL as u32
            == crate::PonkoAssetClass::PONKO_ASSET_CLASS_MATERIAL as u32
    );
    assert!(
        CrabbyAssetClass::CRABBY_ASSET_CLASS_SHADER as u32 == crate::PonkoAssetClass::PONKO_ASSET_CLASS_SHADER as u32
    );
    assert!(CrabbyAssetClass::CRABBY_ASSET_CLASS_MESH as u32 == crate::PonkoAssetClass::PONKO_ASSET_CLASS_MESH as u32);
    assert!(
        CrabbyAssetClass::CRABBY_ASSET_CLASS_GPU_BUFFER as u32
            == crate::PonkoAssetClass::PONKO_ASSET_CLASS_GPU_BUFFER as u32
    );
    assert!(
        CrabbyAssetClass::CRABBY_ASSET_CLASS_EFFECT as u32 == crate::PonkoAssetClass::PONKO_ASSET_CLASS_EFFECT as u32
    );
    assert!(
        CrabbyAssetClass::CRABBY_ASSET_CLASS_COMPUTE as u32 == crate::PonkoAssetClass::PONKO_ASSET_CLASS_COMPUTE as u32
    );
    assert!(
        CrabbyAssetClass::CRABBY_ASSET_CLASS_SOUND as u32 == crate::PonkoAssetClass::PONKO_ASSET_CLASS_SOUND as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAssetClass::CRABBY_ASSET_CLASS_COMPUTE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAssetClass::CRABBY_ASSET_CLASS_EFFECT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAssetClass::CRABBY_ASSET_CLASS_GPU_BUFFER;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAssetClass::CRABBY_ASSET_CLASS_MATERIAL;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAssetClass::CRABBY_ASSET_CLASS_MESH;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAssetClass::CRABBY_ASSET_CLASS_RENDER_TARGET;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAssetClass::CRABBY_ASSET_CLASS_SHADER;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAssetClass::CRABBY_ASSET_CLASS_SOUND;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAssetClass::CRABBY_ASSET_CLASS_TEXTURE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAssetClass::CRABBY_ASSET_CLASS_TEXTURE_ARRAY;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyAssetOwner {
    CRABBY_ASSET_OWNER_CODE = 0,

    CRABBY_ASSET_OWNER_FILE = 1,

    CRABBY_ASSET_OWNER_ENGINE = 2,
}
impl From<crate::PonkoAssetOwner> for CrabbyAssetOwner {
    fn from(value: crate::PonkoAssetOwner) -> Self {
        match value {
            crate::PonkoAssetOwner::PONKO_ASSET_OWNER_CODE => Self::CRABBY_ASSET_OWNER_CODE,
            crate::PonkoAssetOwner::PONKO_ASSET_OWNER_FILE => Self::CRABBY_ASSET_OWNER_FILE,
            crate::PonkoAssetOwner::PONKO_ASSET_OWNER_ENGINE => Self::CRABBY_ASSET_OWNER_ENGINE,
        }
    }
}
impl From<CrabbyAssetOwner> for crate::PonkoAssetOwner {
    fn from(value: CrabbyAssetOwner) -> Self {
        match value {
            CrabbyAssetOwner::CRABBY_ASSET_OWNER_CODE => Self::PONKO_ASSET_OWNER_CODE,
            CrabbyAssetOwner::CRABBY_ASSET_OWNER_FILE => Self::PONKO_ASSET_OWNER_FILE,
            CrabbyAssetOwner::CRABBY_ASSET_OWNER_ENGINE => Self::PONKO_ASSET_OWNER_ENGINE,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyAssetOwner>() == std::mem::size_of::<crate::PonkoAssetOwner>());
    assert!(std::mem::align_of::<CrabbyAssetOwner>() == std::mem::align_of::<crate::PonkoAssetOwner>());
    assert!(CrabbyAssetOwner::CRABBY_ASSET_OWNER_CODE as u32 == crate::PonkoAssetOwner::PONKO_ASSET_OWNER_CODE as u32);
    assert!(CrabbyAssetOwner::CRABBY_ASSET_OWNER_FILE as u32 == crate::PonkoAssetOwner::PONKO_ASSET_OWNER_FILE as u32);
    assert!(
        CrabbyAssetOwner::CRABBY_ASSET_OWNER_ENGINE as u32 == crate::PonkoAssetOwner::PONKO_ASSET_OWNER_ENGINE as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoAssetCount as CrabbyAssetCount;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoAssetRecord as CrabbyAssetRecord;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAssetOwner::CRABBY_ASSET_OWNER_CODE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAssetOwner::CRABBY_ASSET_OWNER_ENGINE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAssetOwner::CRABBY_ASSET_OWNER_FILE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyKey {
    CRABBY_KEY_A,

    CRABBY_KEY_B,

    CRABBY_KEY_C,

    CRABBY_KEY_D,

    CRABBY_KEY_E,

    CRABBY_KEY_F,

    CRABBY_KEY_G,

    CRABBY_KEY_H,

    CRABBY_KEY_I,

    CRABBY_KEY_J,

    CRABBY_KEY_K,

    CRABBY_KEY_L,

    CRABBY_KEY_M,

    CRABBY_KEY_N,

    CRABBY_KEY_O,

    CRABBY_KEY_P,

    CRABBY_KEY_Q,

    CRABBY_KEY_R,

    CRABBY_KEY_S,

    CRABBY_KEY_T,

    CRABBY_KEY_U,

    CRABBY_KEY_V,

    CRABBY_KEY_W,

    CRABBY_KEY_X,

    CRABBY_KEY_Y,

    CRABBY_KEY_Z,

    CRABBY_KEY_NUM_0,

    CRABBY_KEY_NUM_1,

    CRABBY_KEY_NUM_2,

    CRABBY_KEY_NUM_3,

    CRABBY_KEY_NUM_4,

    CRABBY_KEY_NUM_5,

    CRABBY_KEY_NUM_6,

    CRABBY_KEY_NUM_7,

    CRABBY_KEY_NUM_8,

    CRABBY_KEY_NUM_9,

    CRABBY_KEY_F1,

    CRABBY_KEY_F2,

    CRABBY_KEY_F3,

    CRABBY_KEY_F4,

    CRABBY_KEY_F5,

    CRABBY_KEY_F6,

    CRABBY_KEY_F7,

    CRABBY_KEY_F8,

    CRABBY_KEY_F9,

    CRABBY_KEY_F10,

    CRABBY_KEY_F11,

    CRABBY_KEY_F12,

    CRABBY_KEY_UP,

    CRABBY_KEY_DOWN,

    CRABBY_KEY_LEFT,

    CRABBY_KEY_RIGHT,

    CRABBY_KEY_SPACE,

    CRABBY_KEY_ENTER,

    CRABBY_KEY_ESCAPE,

    CRABBY_KEY_TAB,

    CRABBY_KEY_BACKSPACE,

    CRABBY_KEY_CAPS_LOCK,

    CRABBY_KEY_INSERT,

    CRABBY_KEY_DELETE,

    CRABBY_KEY_HOME,

    CRABBY_KEY_END,

    CRABBY_KEY_PAGE_UP,

    CRABBY_KEY_PAGE_DOWN,

    CRABBY_KEY_LEFT_SHIFT,

    CRABBY_KEY_RIGHT_SHIFT,

    CRABBY_KEY_LEFT_CTRL,

    CRABBY_KEY_RIGHT_CTRL,

    CRABBY_KEY_LEFT_ALT,

    CRABBY_KEY_RIGHT_ALT,

    CRABBY_KEY_LEFT_SUPER,

    CRABBY_KEY_RIGHT_SUPER,

    CRABBY_KEY_MINUS,

    CRABBY_KEY_EQUAL,

    CRABBY_KEY_LEFT_BRACKET,

    CRABBY_KEY_RIGHT_BRACKET,

    CRABBY_KEY_BACKSLASH,

    CRABBY_KEY_SEMICOLON,

    CRABBY_KEY_APOSTROPHE,

    CRABBY_KEY_BACKQUOTE,

    CRABBY_KEY_COMMA,

    CRABBY_KEY_PERIOD,

    CRABBY_KEY_SLASH,

    CRABBY_KEY_NUMPAD_0,

    CRABBY_KEY_NUMPAD_1,

    CRABBY_KEY_NUMPAD_2,

    CRABBY_KEY_NUMPAD_3,

    CRABBY_KEY_NUMPAD_4,

    CRABBY_KEY_NUMPAD_5,

    CRABBY_KEY_NUMPAD_6,

    CRABBY_KEY_NUMPAD_7,

    CRABBY_KEY_NUMPAD_8,

    CRABBY_KEY_NUMPAD_9,

    CRABBY_KEY_NUMPAD_ADD,

    CRABBY_KEY_NUMPAD_SUBTRACT,

    CRABBY_KEY_NUMPAD_MULTIPLY,

    CRABBY_KEY_NUMPAD_DIVIDE,

    CRABBY_KEY_NUMPAD_DECIMAL,

    CRABBY_KEY_NUMPAD_ENTER,
}
impl From<crate::PonkoKey> for CrabbyKey {
    fn from(value: crate::PonkoKey) -> Self {
        match value {
            crate::PonkoKey::PONKO_KEY_A => Self::CRABBY_KEY_A,
            crate::PonkoKey::PONKO_KEY_B => Self::CRABBY_KEY_B,
            crate::PonkoKey::PONKO_KEY_C => Self::CRABBY_KEY_C,
            crate::PonkoKey::PONKO_KEY_D => Self::CRABBY_KEY_D,
            crate::PonkoKey::PONKO_KEY_E => Self::CRABBY_KEY_E,
            crate::PonkoKey::PONKO_KEY_F => Self::CRABBY_KEY_F,
            crate::PonkoKey::PONKO_KEY_G => Self::CRABBY_KEY_G,
            crate::PonkoKey::PONKO_KEY_H => Self::CRABBY_KEY_H,
            crate::PonkoKey::PONKO_KEY_I => Self::CRABBY_KEY_I,
            crate::PonkoKey::PONKO_KEY_J => Self::CRABBY_KEY_J,
            crate::PonkoKey::PONKO_KEY_K => Self::CRABBY_KEY_K,
            crate::PonkoKey::PONKO_KEY_L => Self::CRABBY_KEY_L,
            crate::PonkoKey::PONKO_KEY_M => Self::CRABBY_KEY_M,
            crate::PonkoKey::PONKO_KEY_N => Self::CRABBY_KEY_N,
            crate::PonkoKey::PONKO_KEY_O => Self::CRABBY_KEY_O,
            crate::PonkoKey::PONKO_KEY_P => Self::CRABBY_KEY_P,
            crate::PonkoKey::PONKO_KEY_Q => Self::CRABBY_KEY_Q,
            crate::PonkoKey::PONKO_KEY_R => Self::CRABBY_KEY_R,
            crate::PonkoKey::PONKO_KEY_S => Self::CRABBY_KEY_S,
            crate::PonkoKey::PONKO_KEY_T => Self::CRABBY_KEY_T,
            crate::PonkoKey::PONKO_KEY_U => Self::CRABBY_KEY_U,
            crate::PonkoKey::PONKO_KEY_V => Self::CRABBY_KEY_V,
            crate::PonkoKey::PONKO_KEY_W => Self::CRABBY_KEY_W,
            crate::PonkoKey::PONKO_KEY_X => Self::CRABBY_KEY_X,
            crate::PonkoKey::PONKO_KEY_Y => Self::CRABBY_KEY_Y,
            crate::PonkoKey::PONKO_KEY_Z => Self::CRABBY_KEY_Z,
            crate::PonkoKey::PONKO_KEY_NUM_0 => Self::CRABBY_KEY_NUM_0,
            crate::PonkoKey::PONKO_KEY_NUM_1 => Self::CRABBY_KEY_NUM_1,
            crate::PonkoKey::PONKO_KEY_NUM_2 => Self::CRABBY_KEY_NUM_2,
            crate::PonkoKey::PONKO_KEY_NUM_3 => Self::CRABBY_KEY_NUM_3,
            crate::PonkoKey::PONKO_KEY_NUM_4 => Self::CRABBY_KEY_NUM_4,
            crate::PonkoKey::PONKO_KEY_NUM_5 => Self::CRABBY_KEY_NUM_5,
            crate::PonkoKey::PONKO_KEY_NUM_6 => Self::CRABBY_KEY_NUM_6,
            crate::PonkoKey::PONKO_KEY_NUM_7 => Self::CRABBY_KEY_NUM_7,
            crate::PonkoKey::PONKO_KEY_NUM_8 => Self::CRABBY_KEY_NUM_8,
            crate::PonkoKey::PONKO_KEY_NUM_9 => Self::CRABBY_KEY_NUM_9,
            crate::PonkoKey::PONKO_KEY_F1 => Self::CRABBY_KEY_F1,
            crate::PonkoKey::PONKO_KEY_F2 => Self::CRABBY_KEY_F2,
            crate::PonkoKey::PONKO_KEY_F3 => Self::CRABBY_KEY_F3,
            crate::PonkoKey::PONKO_KEY_F4 => Self::CRABBY_KEY_F4,
            crate::PonkoKey::PONKO_KEY_F5 => Self::CRABBY_KEY_F5,
            crate::PonkoKey::PONKO_KEY_F6 => Self::CRABBY_KEY_F6,
            crate::PonkoKey::PONKO_KEY_F7 => Self::CRABBY_KEY_F7,
            crate::PonkoKey::PONKO_KEY_F8 => Self::CRABBY_KEY_F8,
            crate::PonkoKey::PONKO_KEY_F9 => Self::CRABBY_KEY_F9,
            crate::PonkoKey::PONKO_KEY_F10 => Self::CRABBY_KEY_F10,
            crate::PonkoKey::PONKO_KEY_F11 => Self::CRABBY_KEY_F11,
            crate::PonkoKey::PONKO_KEY_F12 => Self::CRABBY_KEY_F12,
            crate::PonkoKey::PONKO_KEY_UP => Self::CRABBY_KEY_UP,
            crate::PonkoKey::PONKO_KEY_DOWN => Self::CRABBY_KEY_DOWN,
            crate::PonkoKey::PONKO_KEY_LEFT => Self::CRABBY_KEY_LEFT,
            crate::PonkoKey::PONKO_KEY_RIGHT => Self::CRABBY_KEY_RIGHT,
            crate::PonkoKey::PONKO_KEY_SPACE => Self::CRABBY_KEY_SPACE,
            crate::PonkoKey::PONKO_KEY_ENTER => Self::CRABBY_KEY_ENTER,
            crate::PonkoKey::PONKO_KEY_ESCAPE => Self::CRABBY_KEY_ESCAPE,
            crate::PonkoKey::PONKO_KEY_TAB => Self::CRABBY_KEY_TAB,
            crate::PonkoKey::PONKO_KEY_BACKSPACE => Self::CRABBY_KEY_BACKSPACE,
            crate::PonkoKey::PONKO_KEY_CAPS_LOCK => Self::CRABBY_KEY_CAPS_LOCK,
            crate::PonkoKey::PONKO_KEY_INSERT => Self::CRABBY_KEY_INSERT,
            crate::PonkoKey::PONKO_KEY_DELETE => Self::CRABBY_KEY_DELETE,
            crate::PonkoKey::PONKO_KEY_HOME => Self::CRABBY_KEY_HOME,
            crate::PonkoKey::PONKO_KEY_END => Self::CRABBY_KEY_END,
            crate::PonkoKey::PONKO_KEY_PAGE_UP => Self::CRABBY_KEY_PAGE_UP,
            crate::PonkoKey::PONKO_KEY_PAGE_DOWN => Self::CRABBY_KEY_PAGE_DOWN,
            crate::PonkoKey::PONKO_KEY_LEFT_SHIFT => Self::CRABBY_KEY_LEFT_SHIFT,
            crate::PonkoKey::PONKO_KEY_RIGHT_SHIFT => Self::CRABBY_KEY_RIGHT_SHIFT,
            crate::PonkoKey::PONKO_KEY_LEFT_CTRL => Self::CRABBY_KEY_LEFT_CTRL,
            crate::PonkoKey::PONKO_KEY_RIGHT_CTRL => Self::CRABBY_KEY_RIGHT_CTRL,
            crate::PonkoKey::PONKO_KEY_LEFT_ALT => Self::CRABBY_KEY_LEFT_ALT,
            crate::PonkoKey::PONKO_KEY_RIGHT_ALT => Self::CRABBY_KEY_RIGHT_ALT,
            crate::PonkoKey::PONKO_KEY_LEFT_SUPER => Self::CRABBY_KEY_LEFT_SUPER,
            crate::PonkoKey::PONKO_KEY_RIGHT_SUPER => Self::CRABBY_KEY_RIGHT_SUPER,
            crate::PonkoKey::PONKO_KEY_MINUS => Self::CRABBY_KEY_MINUS,
            crate::PonkoKey::PONKO_KEY_EQUAL => Self::CRABBY_KEY_EQUAL,
            crate::PonkoKey::PONKO_KEY_LEFT_BRACKET => Self::CRABBY_KEY_LEFT_BRACKET,
            crate::PonkoKey::PONKO_KEY_RIGHT_BRACKET => Self::CRABBY_KEY_RIGHT_BRACKET,
            crate::PonkoKey::PONKO_KEY_BACKSLASH => Self::CRABBY_KEY_BACKSLASH,
            crate::PonkoKey::PONKO_KEY_SEMICOLON => Self::CRABBY_KEY_SEMICOLON,
            crate::PonkoKey::PONKO_KEY_APOSTROPHE => Self::CRABBY_KEY_APOSTROPHE,
            crate::PonkoKey::PONKO_KEY_BACKQUOTE => Self::CRABBY_KEY_BACKQUOTE,
            crate::PonkoKey::PONKO_KEY_COMMA => Self::CRABBY_KEY_COMMA,
            crate::PonkoKey::PONKO_KEY_PERIOD => Self::CRABBY_KEY_PERIOD,
            crate::PonkoKey::PONKO_KEY_SLASH => Self::CRABBY_KEY_SLASH,
            crate::PonkoKey::PONKO_KEY_NUMPAD_0 => Self::CRABBY_KEY_NUMPAD_0,
            crate::PonkoKey::PONKO_KEY_NUMPAD_1 => Self::CRABBY_KEY_NUMPAD_1,
            crate::PonkoKey::PONKO_KEY_NUMPAD_2 => Self::CRABBY_KEY_NUMPAD_2,
            crate::PonkoKey::PONKO_KEY_NUMPAD_3 => Self::CRABBY_KEY_NUMPAD_3,
            crate::PonkoKey::PONKO_KEY_NUMPAD_4 => Self::CRABBY_KEY_NUMPAD_4,
            crate::PonkoKey::PONKO_KEY_NUMPAD_5 => Self::CRABBY_KEY_NUMPAD_5,
            crate::PonkoKey::PONKO_KEY_NUMPAD_6 => Self::CRABBY_KEY_NUMPAD_6,
            crate::PonkoKey::PONKO_KEY_NUMPAD_7 => Self::CRABBY_KEY_NUMPAD_7,
            crate::PonkoKey::PONKO_KEY_NUMPAD_8 => Self::CRABBY_KEY_NUMPAD_8,
            crate::PonkoKey::PONKO_KEY_NUMPAD_9 => Self::CRABBY_KEY_NUMPAD_9,
            crate::PonkoKey::PONKO_KEY_NUMPAD_ADD => Self::CRABBY_KEY_NUMPAD_ADD,
            crate::PonkoKey::PONKO_KEY_NUMPAD_SUBTRACT => Self::CRABBY_KEY_NUMPAD_SUBTRACT,
            crate::PonkoKey::PONKO_KEY_NUMPAD_MULTIPLY => Self::CRABBY_KEY_NUMPAD_MULTIPLY,
            crate::PonkoKey::PONKO_KEY_NUMPAD_DIVIDE => Self::CRABBY_KEY_NUMPAD_DIVIDE,
            crate::PonkoKey::PONKO_KEY_NUMPAD_DECIMAL => Self::CRABBY_KEY_NUMPAD_DECIMAL,
            crate::PonkoKey::PONKO_KEY_NUMPAD_ENTER => Self::CRABBY_KEY_NUMPAD_ENTER,
        }
    }
}
impl From<CrabbyKey> for crate::PonkoKey {
    fn from(value: CrabbyKey) -> Self {
        match value {
            CrabbyKey::CRABBY_KEY_A => Self::PONKO_KEY_A,
            CrabbyKey::CRABBY_KEY_B => Self::PONKO_KEY_B,
            CrabbyKey::CRABBY_KEY_C => Self::PONKO_KEY_C,
            CrabbyKey::CRABBY_KEY_D => Self::PONKO_KEY_D,
            CrabbyKey::CRABBY_KEY_E => Self::PONKO_KEY_E,
            CrabbyKey::CRABBY_KEY_F => Self::PONKO_KEY_F,
            CrabbyKey::CRABBY_KEY_G => Self::PONKO_KEY_G,
            CrabbyKey::CRABBY_KEY_H => Self::PONKO_KEY_H,
            CrabbyKey::CRABBY_KEY_I => Self::PONKO_KEY_I,
            CrabbyKey::CRABBY_KEY_J => Self::PONKO_KEY_J,
            CrabbyKey::CRABBY_KEY_K => Self::PONKO_KEY_K,
            CrabbyKey::CRABBY_KEY_L => Self::PONKO_KEY_L,
            CrabbyKey::CRABBY_KEY_M => Self::PONKO_KEY_M,
            CrabbyKey::CRABBY_KEY_N => Self::PONKO_KEY_N,
            CrabbyKey::CRABBY_KEY_O => Self::PONKO_KEY_O,
            CrabbyKey::CRABBY_KEY_P => Self::PONKO_KEY_P,
            CrabbyKey::CRABBY_KEY_Q => Self::PONKO_KEY_Q,
            CrabbyKey::CRABBY_KEY_R => Self::PONKO_KEY_R,
            CrabbyKey::CRABBY_KEY_S => Self::PONKO_KEY_S,
            CrabbyKey::CRABBY_KEY_T => Self::PONKO_KEY_T,
            CrabbyKey::CRABBY_KEY_U => Self::PONKO_KEY_U,
            CrabbyKey::CRABBY_KEY_V => Self::PONKO_KEY_V,
            CrabbyKey::CRABBY_KEY_W => Self::PONKO_KEY_W,
            CrabbyKey::CRABBY_KEY_X => Self::PONKO_KEY_X,
            CrabbyKey::CRABBY_KEY_Y => Self::PONKO_KEY_Y,
            CrabbyKey::CRABBY_KEY_Z => Self::PONKO_KEY_Z,
            CrabbyKey::CRABBY_KEY_NUM_0 => Self::PONKO_KEY_NUM_0,
            CrabbyKey::CRABBY_KEY_NUM_1 => Self::PONKO_KEY_NUM_1,
            CrabbyKey::CRABBY_KEY_NUM_2 => Self::PONKO_KEY_NUM_2,
            CrabbyKey::CRABBY_KEY_NUM_3 => Self::PONKO_KEY_NUM_3,
            CrabbyKey::CRABBY_KEY_NUM_4 => Self::PONKO_KEY_NUM_4,
            CrabbyKey::CRABBY_KEY_NUM_5 => Self::PONKO_KEY_NUM_5,
            CrabbyKey::CRABBY_KEY_NUM_6 => Self::PONKO_KEY_NUM_6,
            CrabbyKey::CRABBY_KEY_NUM_7 => Self::PONKO_KEY_NUM_7,
            CrabbyKey::CRABBY_KEY_NUM_8 => Self::PONKO_KEY_NUM_8,
            CrabbyKey::CRABBY_KEY_NUM_9 => Self::PONKO_KEY_NUM_9,
            CrabbyKey::CRABBY_KEY_F1 => Self::PONKO_KEY_F1,
            CrabbyKey::CRABBY_KEY_F2 => Self::PONKO_KEY_F2,
            CrabbyKey::CRABBY_KEY_F3 => Self::PONKO_KEY_F3,
            CrabbyKey::CRABBY_KEY_F4 => Self::PONKO_KEY_F4,
            CrabbyKey::CRABBY_KEY_F5 => Self::PONKO_KEY_F5,
            CrabbyKey::CRABBY_KEY_F6 => Self::PONKO_KEY_F6,
            CrabbyKey::CRABBY_KEY_F7 => Self::PONKO_KEY_F7,
            CrabbyKey::CRABBY_KEY_F8 => Self::PONKO_KEY_F8,
            CrabbyKey::CRABBY_KEY_F9 => Self::PONKO_KEY_F9,
            CrabbyKey::CRABBY_KEY_F10 => Self::PONKO_KEY_F10,
            CrabbyKey::CRABBY_KEY_F11 => Self::PONKO_KEY_F11,
            CrabbyKey::CRABBY_KEY_F12 => Self::PONKO_KEY_F12,
            CrabbyKey::CRABBY_KEY_UP => Self::PONKO_KEY_UP,
            CrabbyKey::CRABBY_KEY_DOWN => Self::PONKO_KEY_DOWN,
            CrabbyKey::CRABBY_KEY_LEFT => Self::PONKO_KEY_LEFT,
            CrabbyKey::CRABBY_KEY_RIGHT => Self::PONKO_KEY_RIGHT,
            CrabbyKey::CRABBY_KEY_SPACE => Self::PONKO_KEY_SPACE,
            CrabbyKey::CRABBY_KEY_ENTER => Self::PONKO_KEY_ENTER,
            CrabbyKey::CRABBY_KEY_ESCAPE => Self::PONKO_KEY_ESCAPE,
            CrabbyKey::CRABBY_KEY_TAB => Self::PONKO_KEY_TAB,
            CrabbyKey::CRABBY_KEY_BACKSPACE => Self::PONKO_KEY_BACKSPACE,
            CrabbyKey::CRABBY_KEY_CAPS_LOCK => Self::PONKO_KEY_CAPS_LOCK,
            CrabbyKey::CRABBY_KEY_INSERT => Self::PONKO_KEY_INSERT,
            CrabbyKey::CRABBY_KEY_DELETE => Self::PONKO_KEY_DELETE,
            CrabbyKey::CRABBY_KEY_HOME => Self::PONKO_KEY_HOME,
            CrabbyKey::CRABBY_KEY_END => Self::PONKO_KEY_END,
            CrabbyKey::CRABBY_KEY_PAGE_UP => Self::PONKO_KEY_PAGE_UP,
            CrabbyKey::CRABBY_KEY_PAGE_DOWN => Self::PONKO_KEY_PAGE_DOWN,
            CrabbyKey::CRABBY_KEY_LEFT_SHIFT => Self::PONKO_KEY_LEFT_SHIFT,
            CrabbyKey::CRABBY_KEY_RIGHT_SHIFT => Self::PONKO_KEY_RIGHT_SHIFT,
            CrabbyKey::CRABBY_KEY_LEFT_CTRL => Self::PONKO_KEY_LEFT_CTRL,
            CrabbyKey::CRABBY_KEY_RIGHT_CTRL => Self::PONKO_KEY_RIGHT_CTRL,
            CrabbyKey::CRABBY_KEY_LEFT_ALT => Self::PONKO_KEY_LEFT_ALT,
            CrabbyKey::CRABBY_KEY_RIGHT_ALT => Self::PONKO_KEY_RIGHT_ALT,
            CrabbyKey::CRABBY_KEY_LEFT_SUPER => Self::PONKO_KEY_LEFT_SUPER,
            CrabbyKey::CRABBY_KEY_RIGHT_SUPER => Self::PONKO_KEY_RIGHT_SUPER,
            CrabbyKey::CRABBY_KEY_MINUS => Self::PONKO_KEY_MINUS,
            CrabbyKey::CRABBY_KEY_EQUAL => Self::PONKO_KEY_EQUAL,
            CrabbyKey::CRABBY_KEY_LEFT_BRACKET => Self::PONKO_KEY_LEFT_BRACKET,
            CrabbyKey::CRABBY_KEY_RIGHT_BRACKET => Self::PONKO_KEY_RIGHT_BRACKET,
            CrabbyKey::CRABBY_KEY_BACKSLASH => Self::PONKO_KEY_BACKSLASH,
            CrabbyKey::CRABBY_KEY_SEMICOLON => Self::PONKO_KEY_SEMICOLON,
            CrabbyKey::CRABBY_KEY_APOSTROPHE => Self::PONKO_KEY_APOSTROPHE,
            CrabbyKey::CRABBY_KEY_BACKQUOTE => Self::PONKO_KEY_BACKQUOTE,
            CrabbyKey::CRABBY_KEY_COMMA => Self::PONKO_KEY_COMMA,
            CrabbyKey::CRABBY_KEY_PERIOD => Self::PONKO_KEY_PERIOD,
            CrabbyKey::CRABBY_KEY_SLASH => Self::PONKO_KEY_SLASH,
            CrabbyKey::CRABBY_KEY_NUMPAD_0 => Self::PONKO_KEY_NUMPAD_0,
            CrabbyKey::CRABBY_KEY_NUMPAD_1 => Self::PONKO_KEY_NUMPAD_1,
            CrabbyKey::CRABBY_KEY_NUMPAD_2 => Self::PONKO_KEY_NUMPAD_2,
            CrabbyKey::CRABBY_KEY_NUMPAD_3 => Self::PONKO_KEY_NUMPAD_3,
            CrabbyKey::CRABBY_KEY_NUMPAD_4 => Self::PONKO_KEY_NUMPAD_4,
            CrabbyKey::CRABBY_KEY_NUMPAD_5 => Self::PONKO_KEY_NUMPAD_5,
            CrabbyKey::CRABBY_KEY_NUMPAD_6 => Self::PONKO_KEY_NUMPAD_6,
            CrabbyKey::CRABBY_KEY_NUMPAD_7 => Self::PONKO_KEY_NUMPAD_7,
            CrabbyKey::CRABBY_KEY_NUMPAD_8 => Self::PONKO_KEY_NUMPAD_8,
            CrabbyKey::CRABBY_KEY_NUMPAD_9 => Self::PONKO_KEY_NUMPAD_9,
            CrabbyKey::CRABBY_KEY_NUMPAD_ADD => Self::PONKO_KEY_NUMPAD_ADD,
            CrabbyKey::CRABBY_KEY_NUMPAD_SUBTRACT => Self::PONKO_KEY_NUMPAD_SUBTRACT,
            CrabbyKey::CRABBY_KEY_NUMPAD_MULTIPLY => Self::PONKO_KEY_NUMPAD_MULTIPLY,
            CrabbyKey::CRABBY_KEY_NUMPAD_DIVIDE => Self::PONKO_KEY_NUMPAD_DIVIDE,
            CrabbyKey::CRABBY_KEY_NUMPAD_DECIMAL => Self::PONKO_KEY_NUMPAD_DECIMAL,
            CrabbyKey::CRABBY_KEY_NUMPAD_ENTER => Self::PONKO_KEY_NUMPAD_ENTER,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyKey>() == std::mem::size_of::<crate::PonkoKey>());
    assert!(std::mem::align_of::<CrabbyKey>() == std::mem::align_of::<crate::PonkoKey>());
    assert!(CrabbyKey::CRABBY_KEY_A as u32 == crate::PonkoKey::PONKO_KEY_A as u32);
    assert!(CrabbyKey::CRABBY_KEY_B as u32 == crate::PonkoKey::PONKO_KEY_B as u32);
    assert!(CrabbyKey::CRABBY_KEY_C as u32 == crate::PonkoKey::PONKO_KEY_C as u32);
    assert!(CrabbyKey::CRABBY_KEY_D as u32 == crate::PonkoKey::PONKO_KEY_D as u32);
    assert!(CrabbyKey::CRABBY_KEY_E as u32 == crate::PonkoKey::PONKO_KEY_E as u32);
    assert!(CrabbyKey::CRABBY_KEY_F as u32 == crate::PonkoKey::PONKO_KEY_F as u32);
    assert!(CrabbyKey::CRABBY_KEY_G as u32 == crate::PonkoKey::PONKO_KEY_G as u32);
    assert!(CrabbyKey::CRABBY_KEY_H as u32 == crate::PonkoKey::PONKO_KEY_H as u32);
    assert!(CrabbyKey::CRABBY_KEY_I as u32 == crate::PonkoKey::PONKO_KEY_I as u32);
    assert!(CrabbyKey::CRABBY_KEY_J as u32 == crate::PonkoKey::PONKO_KEY_J as u32);
    assert!(CrabbyKey::CRABBY_KEY_K as u32 == crate::PonkoKey::PONKO_KEY_K as u32);
    assert!(CrabbyKey::CRABBY_KEY_L as u32 == crate::PonkoKey::PONKO_KEY_L as u32);
    assert!(CrabbyKey::CRABBY_KEY_M as u32 == crate::PonkoKey::PONKO_KEY_M as u32);
    assert!(CrabbyKey::CRABBY_KEY_N as u32 == crate::PonkoKey::PONKO_KEY_N as u32);
    assert!(CrabbyKey::CRABBY_KEY_O as u32 == crate::PonkoKey::PONKO_KEY_O as u32);
    assert!(CrabbyKey::CRABBY_KEY_P as u32 == crate::PonkoKey::PONKO_KEY_P as u32);
    assert!(CrabbyKey::CRABBY_KEY_Q as u32 == crate::PonkoKey::PONKO_KEY_Q as u32);
    assert!(CrabbyKey::CRABBY_KEY_R as u32 == crate::PonkoKey::PONKO_KEY_R as u32);
    assert!(CrabbyKey::CRABBY_KEY_S as u32 == crate::PonkoKey::PONKO_KEY_S as u32);
    assert!(CrabbyKey::CRABBY_KEY_T as u32 == crate::PonkoKey::PONKO_KEY_T as u32);
    assert!(CrabbyKey::CRABBY_KEY_U as u32 == crate::PonkoKey::PONKO_KEY_U as u32);
    assert!(CrabbyKey::CRABBY_KEY_V as u32 == crate::PonkoKey::PONKO_KEY_V as u32);
    assert!(CrabbyKey::CRABBY_KEY_W as u32 == crate::PonkoKey::PONKO_KEY_W as u32);
    assert!(CrabbyKey::CRABBY_KEY_X as u32 == crate::PonkoKey::PONKO_KEY_X as u32);
    assert!(CrabbyKey::CRABBY_KEY_Y as u32 == crate::PonkoKey::PONKO_KEY_Y as u32);
    assert!(CrabbyKey::CRABBY_KEY_Z as u32 == crate::PonkoKey::PONKO_KEY_Z as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUM_0 as u32 == crate::PonkoKey::PONKO_KEY_NUM_0 as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUM_1 as u32 == crate::PonkoKey::PONKO_KEY_NUM_1 as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUM_2 as u32 == crate::PonkoKey::PONKO_KEY_NUM_2 as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUM_3 as u32 == crate::PonkoKey::PONKO_KEY_NUM_3 as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUM_4 as u32 == crate::PonkoKey::PONKO_KEY_NUM_4 as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUM_5 as u32 == crate::PonkoKey::PONKO_KEY_NUM_5 as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUM_6 as u32 == crate::PonkoKey::PONKO_KEY_NUM_6 as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUM_7 as u32 == crate::PonkoKey::PONKO_KEY_NUM_7 as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUM_8 as u32 == crate::PonkoKey::PONKO_KEY_NUM_8 as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUM_9 as u32 == crate::PonkoKey::PONKO_KEY_NUM_9 as u32);
    assert!(CrabbyKey::CRABBY_KEY_F1 as u32 == crate::PonkoKey::PONKO_KEY_F1 as u32);
    assert!(CrabbyKey::CRABBY_KEY_F2 as u32 == crate::PonkoKey::PONKO_KEY_F2 as u32);
    assert!(CrabbyKey::CRABBY_KEY_F3 as u32 == crate::PonkoKey::PONKO_KEY_F3 as u32);
    assert!(CrabbyKey::CRABBY_KEY_F4 as u32 == crate::PonkoKey::PONKO_KEY_F4 as u32);
    assert!(CrabbyKey::CRABBY_KEY_F5 as u32 == crate::PonkoKey::PONKO_KEY_F5 as u32);
    assert!(CrabbyKey::CRABBY_KEY_F6 as u32 == crate::PonkoKey::PONKO_KEY_F6 as u32);
    assert!(CrabbyKey::CRABBY_KEY_F7 as u32 == crate::PonkoKey::PONKO_KEY_F7 as u32);
    assert!(CrabbyKey::CRABBY_KEY_F8 as u32 == crate::PonkoKey::PONKO_KEY_F8 as u32);
    assert!(CrabbyKey::CRABBY_KEY_F9 as u32 == crate::PonkoKey::PONKO_KEY_F9 as u32);
    assert!(CrabbyKey::CRABBY_KEY_F10 as u32 == crate::PonkoKey::PONKO_KEY_F10 as u32);
    assert!(CrabbyKey::CRABBY_KEY_F11 as u32 == crate::PonkoKey::PONKO_KEY_F11 as u32);
    assert!(CrabbyKey::CRABBY_KEY_F12 as u32 == crate::PonkoKey::PONKO_KEY_F12 as u32);
    assert!(CrabbyKey::CRABBY_KEY_UP as u32 == crate::PonkoKey::PONKO_KEY_UP as u32);
    assert!(CrabbyKey::CRABBY_KEY_DOWN as u32 == crate::PonkoKey::PONKO_KEY_DOWN as u32);
    assert!(CrabbyKey::CRABBY_KEY_LEFT as u32 == crate::PonkoKey::PONKO_KEY_LEFT as u32);
    assert!(CrabbyKey::CRABBY_KEY_RIGHT as u32 == crate::PonkoKey::PONKO_KEY_RIGHT as u32);
    assert!(CrabbyKey::CRABBY_KEY_SPACE as u32 == crate::PonkoKey::PONKO_KEY_SPACE as u32);
    assert!(CrabbyKey::CRABBY_KEY_ENTER as u32 == crate::PonkoKey::PONKO_KEY_ENTER as u32);
    assert!(CrabbyKey::CRABBY_KEY_ESCAPE as u32 == crate::PonkoKey::PONKO_KEY_ESCAPE as u32);
    assert!(CrabbyKey::CRABBY_KEY_TAB as u32 == crate::PonkoKey::PONKO_KEY_TAB as u32);
    assert!(CrabbyKey::CRABBY_KEY_BACKSPACE as u32 == crate::PonkoKey::PONKO_KEY_BACKSPACE as u32);
    assert!(CrabbyKey::CRABBY_KEY_CAPS_LOCK as u32 == crate::PonkoKey::PONKO_KEY_CAPS_LOCK as u32);
    assert!(CrabbyKey::CRABBY_KEY_INSERT as u32 == crate::PonkoKey::PONKO_KEY_INSERT as u32);
    assert!(CrabbyKey::CRABBY_KEY_DELETE as u32 == crate::PonkoKey::PONKO_KEY_DELETE as u32);
    assert!(CrabbyKey::CRABBY_KEY_HOME as u32 == crate::PonkoKey::PONKO_KEY_HOME as u32);
    assert!(CrabbyKey::CRABBY_KEY_END as u32 == crate::PonkoKey::PONKO_KEY_END as u32);
    assert!(CrabbyKey::CRABBY_KEY_PAGE_UP as u32 == crate::PonkoKey::PONKO_KEY_PAGE_UP as u32);
    assert!(CrabbyKey::CRABBY_KEY_PAGE_DOWN as u32 == crate::PonkoKey::PONKO_KEY_PAGE_DOWN as u32);
    assert!(CrabbyKey::CRABBY_KEY_LEFT_SHIFT as u32 == crate::PonkoKey::PONKO_KEY_LEFT_SHIFT as u32);
    assert!(CrabbyKey::CRABBY_KEY_RIGHT_SHIFT as u32 == crate::PonkoKey::PONKO_KEY_RIGHT_SHIFT as u32);
    assert!(CrabbyKey::CRABBY_KEY_LEFT_CTRL as u32 == crate::PonkoKey::PONKO_KEY_LEFT_CTRL as u32);
    assert!(CrabbyKey::CRABBY_KEY_RIGHT_CTRL as u32 == crate::PonkoKey::PONKO_KEY_RIGHT_CTRL as u32);
    assert!(CrabbyKey::CRABBY_KEY_LEFT_ALT as u32 == crate::PonkoKey::PONKO_KEY_LEFT_ALT as u32);
    assert!(CrabbyKey::CRABBY_KEY_RIGHT_ALT as u32 == crate::PonkoKey::PONKO_KEY_RIGHT_ALT as u32);
    assert!(CrabbyKey::CRABBY_KEY_LEFT_SUPER as u32 == crate::PonkoKey::PONKO_KEY_LEFT_SUPER as u32);
    assert!(CrabbyKey::CRABBY_KEY_RIGHT_SUPER as u32 == crate::PonkoKey::PONKO_KEY_RIGHT_SUPER as u32);
    assert!(CrabbyKey::CRABBY_KEY_MINUS as u32 == crate::PonkoKey::PONKO_KEY_MINUS as u32);
    assert!(CrabbyKey::CRABBY_KEY_EQUAL as u32 == crate::PonkoKey::PONKO_KEY_EQUAL as u32);
    assert!(CrabbyKey::CRABBY_KEY_LEFT_BRACKET as u32 == crate::PonkoKey::PONKO_KEY_LEFT_BRACKET as u32);
    assert!(CrabbyKey::CRABBY_KEY_RIGHT_BRACKET as u32 == crate::PonkoKey::PONKO_KEY_RIGHT_BRACKET as u32);
    assert!(CrabbyKey::CRABBY_KEY_BACKSLASH as u32 == crate::PonkoKey::PONKO_KEY_BACKSLASH as u32);
    assert!(CrabbyKey::CRABBY_KEY_SEMICOLON as u32 == crate::PonkoKey::PONKO_KEY_SEMICOLON as u32);
    assert!(CrabbyKey::CRABBY_KEY_APOSTROPHE as u32 == crate::PonkoKey::PONKO_KEY_APOSTROPHE as u32);
    assert!(CrabbyKey::CRABBY_KEY_BACKQUOTE as u32 == crate::PonkoKey::PONKO_KEY_BACKQUOTE as u32);
    assert!(CrabbyKey::CRABBY_KEY_COMMA as u32 == crate::PonkoKey::PONKO_KEY_COMMA as u32);
    assert!(CrabbyKey::CRABBY_KEY_PERIOD as u32 == crate::PonkoKey::PONKO_KEY_PERIOD as u32);
    assert!(CrabbyKey::CRABBY_KEY_SLASH as u32 == crate::PonkoKey::PONKO_KEY_SLASH as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUMPAD_0 as u32 == crate::PonkoKey::PONKO_KEY_NUMPAD_0 as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUMPAD_1 as u32 == crate::PonkoKey::PONKO_KEY_NUMPAD_1 as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUMPAD_2 as u32 == crate::PonkoKey::PONKO_KEY_NUMPAD_2 as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUMPAD_3 as u32 == crate::PonkoKey::PONKO_KEY_NUMPAD_3 as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUMPAD_4 as u32 == crate::PonkoKey::PONKO_KEY_NUMPAD_4 as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUMPAD_5 as u32 == crate::PonkoKey::PONKO_KEY_NUMPAD_5 as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUMPAD_6 as u32 == crate::PonkoKey::PONKO_KEY_NUMPAD_6 as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUMPAD_7 as u32 == crate::PonkoKey::PONKO_KEY_NUMPAD_7 as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUMPAD_8 as u32 == crate::PonkoKey::PONKO_KEY_NUMPAD_8 as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUMPAD_9 as u32 == crate::PonkoKey::PONKO_KEY_NUMPAD_9 as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUMPAD_ADD as u32 == crate::PonkoKey::PONKO_KEY_NUMPAD_ADD as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUMPAD_SUBTRACT as u32 == crate::PonkoKey::PONKO_KEY_NUMPAD_SUBTRACT as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUMPAD_MULTIPLY as u32 == crate::PonkoKey::PONKO_KEY_NUMPAD_MULTIPLY as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUMPAD_DIVIDE as u32 == crate::PonkoKey::PONKO_KEY_NUMPAD_DIVIDE as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUMPAD_DECIMAL as u32 == crate::PonkoKey::PONKO_KEY_NUMPAD_DECIMAL as u32);
    assert!(CrabbyKey::CRABBY_KEY_NUMPAD_ENTER as u32 == crate::PonkoKey::PONKO_KEY_NUMPAD_ENTER as u32);
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_A;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_APOSTROPHE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_B;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_BACKQUOTE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_BACKSLASH;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_BACKSPACE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_C;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_CAPS_LOCK;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_COMMA;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_DELETE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_DOWN;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_E;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_END;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_ENTER;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_EQUAL;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_ESCAPE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_F;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_F1;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_F2;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_F3;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_F4;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_F5;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_F6;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_F7;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_F8;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_F9;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_F10;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_F11;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_F12;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_G;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_H;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_HOME;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_I;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_INSERT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_J;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_K;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_L;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_LEFT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_LEFT_ALT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_LEFT_BRACKET;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_LEFT_CTRL;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_LEFT_SHIFT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_LEFT_SUPER;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_M;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_MINUS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_N;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUM_0;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUM_1;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUM_2;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUM_3;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUM_4;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUM_5;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUM_6;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUM_7;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUM_8;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUM_9;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUMPAD_0;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUMPAD_1;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUMPAD_2;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUMPAD_3;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUMPAD_4;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUMPAD_5;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUMPAD_6;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUMPAD_7;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUMPAD_8;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUMPAD_9;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUMPAD_ADD;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUMPAD_DECIMAL;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUMPAD_DIVIDE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUMPAD_ENTER;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUMPAD_MULTIPLY;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_NUMPAD_SUBTRACT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_O;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_P;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_PAGE_DOWN;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_PAGE_UP;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_PERIOD;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_Q;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_R;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_RIGHT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_RIGHT_ALT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_RIGHT_BRACKET;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_RIGHT_CTRL;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_RIGHT_SHIFT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_RIGHT_SUPER;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_S;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_SEMICOLON;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_SLASH;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_SPACE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_T;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_TAB;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_U;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_UP;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_V;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_W;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_X;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_Y;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyKey::CRABBY_KEY_Z;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyMouseButton {
    CRABBY_MOUSE_LEFT,

    CRABBY_MOUSE_RIGHT,

    CRABBY_MOUSE_MIDDLE,

    CRABBY_MOUSE_BACK,

    CRABBY_MOUSE_FORWARD,
}
impl From<crate::PonkoMouseButton> for CrabbyMouseButton {
    fn from(value: crate::PonkoMouseButton) -> Self {
        match value {
            crate::PonkoMouseButton::PONKO_MOUSE_LEFT => Self::CRABBY_MOUSE_LEFT,
            crate::PonkoMouseButton::PONKO_MOUSE_RIGHT => Self::CRABBY_MOUSE_RIGHT,
            crate::PonkoMouseButton::PONKO_MOUSE_MIDDLE => Self::CRABBY_MOUSE_MIDDLE,
            crate::PonkoMouseButton::PONKO_MOUSE_BACK => Self::CRABBY_MOUSE_BACK,
            crate::PonkoMouseButton::PONKO_MOUSE_FORWARD => Self::CRABBY_MOUSE_FORWARD,
        }
    }
}
impl From<CrabbyMouseButton> for crate::PonkoMouseButton {
    fn from(value: CrabbyMouseButton) -> Self {
        match value {
            CrabbyMouseButton::CRABBY_MOUSE_LEFT => Self::PONKO_MOUSE_LEFT,
            CrabbyMouseButton::CRABBY_MOUSE_RIGHT => Self::PONKO_MOUSE_RIGHT,
            CrabbyMouseButton::CRABBY_MOUSE_MIDDLE => Self::PONKO_MOUSE_MIDDLE,
            CrabbyMouseButton::CRABBY_MOUSE_BACK => Self::PONKO_MOUSE_BACK,
            CrabbyMouseButton::CRABBY_MOUSE_FORWARD => Self::PONKO_MOUSE_FORWARD,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyMouseButton>() == std::mem::size_of::<crate::PonkoMouseButton>());
    assert!(std::mem::align_of::<CrabbyMouseButton>() == std::mem::align_of::<crate::PonkoMouseButton>());
    assert!(CrabbyMouseButton::CRABBY_MOUSE_LEFT as u32 == crate::PonkoMouseButton::PONKO_MOUSE_LEFT as u32);
    assert!(CrabbyMouseButton::CRABBY_MOUSE_RIGHT as u32 == crate::PonkoMouseButton::PONKO_MOUSE_RIGHT as u32);
    assert!(CrabbyMouseButton::CRABBY_MOUSE_MIDDLE as u32 == crate::PonkoMouseButton::PONKO_MOUSE_MIDDLE as u32);
    assert!(CrabbyMouseButton::CRABBY_MOUSE_BACK as u32 == crate::PonkoMouseButton::PONKO_MOUSE_BACK as u32);
    assert!(CrabbyMouseButton::CRABBY_MOUSE_FORWARD as u32 == crate::PonkoMouseButton::PONKO_MOUSE_FORWARD as u32);
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyMouseButton::CRABBY_MOUSE_BACK;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyMouseButton::CRABBY_MOUSE_FORWARD;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyMouseButton::CRABBY_MOUSE_LEFT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyMouseButton::CRABBY_MOUSE_MIDDLE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyMouseButton::CRABBY_MOUSE_RIGHT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyGamepadButton {
    CRABBY_GAMEPAD_SOUTH,

    CRABBY_GAMEPAD_EAST,

    CRABBY_GAMEPAD_NORTH,

    CRABBY_GAMEPAD_WEST,

    CRABBY_GAMEPAD_LEFT_BUMPER,

    CRABBY_GAMEPAD_RIGHT_BUMPER,

    CRABBY_GAMEPAD_LEFT_TRIGGER,

    CRABBY_GAMEPAD_RIGHT_TRIGGER,

    CRABBY_GAMEPAD_SELECT,

    CRABBY_GAMEPAD_START,

    CRABBY_GAMEPAD_LEFT_STICK,

    CRABBY_GAMEPAD_RIGHT_STICK,

    CRABBY_GAMEPAD_DPAD_UP,

    CRABBY_GAMEPAD_DPAD_DOWN,

    CRABBY_GAMEPAD_DPAD_LEFT,

    CRABBY_GAMEPAD_DPAD_RIGHT,
}
impl From<crate::PonkoGamepadButton> for CrabbyGamepadButton {
    fn from(value: crate::PonkoGamepadButton) -> Self {
        match value {
            crate::PonkoGamepadButton::PONKO_GAMEPAD_SOUTH => Self::CRABBY_GAMEPAD_SOUTH,
            crate::PonkoGamepadButton::PONKO_GAMEPAD_EAST => Self::CRABBY_GAMEPAD_EAST,
            crate::PonkoGamepadButton::PONKO_GAMEPAD_NORTH => Self::CRABBY_GAMEPAD_NORTH,
            crate::PonkoGamepadButton::PONKO_GAMEPAD_WEST => Self::CRABBY_GAMEPAD_WEST,
            crate::PonkoGamepadButton::PONKO_GAMEPAD_LEFT_BUMPER => Self::CRABBY_GAMEPAD_LEFT_BUMPER,
            crate::PonkoGamepadButton::PONKO_GAMEPAD_RIGHT_BUMPER => Self::CRABBY_GAMEPAD_RIGHT_BUMPER,
            crate::PonkoGamepadButton::PONKO_GAMEPAD_LEFT_TRIGGER => Self::CRABBY_GAMEPAD_LEFT_TRIGGER,
            crate::PonkoGamepadButton::PONKO_GAMEPAD_RIGHT_TRIGGER => Self::CRABBY_GAMEPAD_RIGHT_TRIGGER,
            crate::PonkoGamepadButton::PONKO_GAMEPAD_SELECT => Self::CRABBY_GAMEPAD_SELECT,
            crate::PonkoGamepadButton::PONKO_GAMEPAD_START => Self::CRABBY_GAMEPAD_START,
            crate::PonkoGamepadButton::PONKO_GAMEPAD_LEFT_STICK => Self::CRABBY_GAMEPAD_LEFT_STICK,
            crate::PonkoGamepadButton::PONKO_GAMEPAD_RIGHT_STICK => Self::CRABBY_GAMEPAD_RIGHT_STICK,
            crate::PonkoGamepadButton::PONKO_GAMEPAD_DPAD_UP => Self::CRABBY_GAMEPAD_DPAD_UP,
            crate::PonkoGamepadButton::PONKO_GAMEPAD_DPAD_DOWN => Self::CRABBY_GAMEPAD_DPAD_DOWN,
            crate::PonkoGamepadButton::PONKO_GAMEPAD_DPAD_LEFT => Self::CRABBY_GAMEPAD_DPAD_LEFT,
            crate::PonkoGamepadButton::PONKO_GAMEPAD_DPAD_RIGHT => Self::CRABBY_GAMEPAD_DPAD_RIGHT,
        }
    }
}
impl From<CrabbyGamepadButton> for crate::PonkoGamepadButton {
    fn from(value: CrabbyGamepadButton) -> Self {
        match value {
            CrabbyGamepadButton::CRABBY_GAMEPAD_SOUTH => Self::PONKO_GAMEPAD_SOUTH,
            CrabbyGamepadButton::CRABBY_GAMEPAD_EAST => Self::PONKO_GAMEPAD_EAST,
            CrabbyGamepadButton::CRABBY_GAMEPAD_NORTH => Self::PONKO_GAMEPAD_NORTH,
            CrabbyGamepadButton::CRABBY_GAMEPAD_WEST => Self::PONKO_GAMEPAD_WEST,
            CrabbyGamepadButton::CRABBY_GAMEPAD_LEFT_BUMPER => Self::PONKO_GAMEPAD_LEFT_BUMPER,
            CrabbyGamepadButton::CRABBY_GAMEPAD_RIGHT_BUMPER => Self::PONKO_GAMEPAD_RIGHT_BUMPER,
            CrabbyGamepadButton::CRABBY_GAMEPAD_LEFT_TRIGGER => Self::PONKO_GAMEPAD_LEFT_TRIGGER,
            CrabbyGamepadButton::CRABBY_GAMEPAD_RIGHT_TRIGGER => Self::PONKO_GAMEPAD_RIGHT_TRIGGER,
            CrabbyGamepadButton::CRABBY_GAMEPAD_SELECT => Self::PONKO_GAMEPAD_SELECT,
            CrabbyGamepadButton::CRABBY_GAMEPAD_START => Self::PONKO_GAMEPAD_START,
            CrabbyGamepadButton::CRABBY_GAMEPAD_LEFT_STICK => Self::PONKO_GAMEPAD_LEFT_STICK,
            CrabbyGamepadButton::CRABBY_GAMEPAD_RIGHT_STICK => Self::PONKO_GAMEPAD_RIGHT_STICK,
            CrabbyGamepadButton::CRABBY_GAMEPAD_DPAD_UP => Self::PONKO_GAMEPAD_DPAD_UP,
            CrabbyGamepadButton::CRABBY_GAMEPAD_DPAD_DOWN => Self::PONKO_GAMEPAD_DPAD_DOWN,
            CrabbyGamepadButton::CRABBY_GAMEPAD_DPAD_LEFT => Self::PONKO_GAMEPAD_DPAD_LEFT,
            CrabbyGamepadButton::CRABBY_GAMEPAD_DPAD_RIGHT => Self::PONKO_GAMEPAD_DPAD_RIGHT,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyGamepadButton>() == std::mem::size_of::<crate::PonkoGamepadButton>());
    assert!(std::mem::align_of::<CrabbyGamepadButton>() == std::mem::align_of::<crate::PonkoGamepadButton>());
    assert!(CrabbyGamepadButton::CRABBY_GAMEPAD_SOUTH as u32 == crate::PonkoGamepadButton::PONKO_GAMEPAD_SOUTH as u32);
    assert!(CrabbyGamepadButton::CRABBY_GAMEPAD_EAST as u32 == crate::PonkoGamepadButton::PONKO_GAMEPAD_EAST as u32);
    assert!(CrabbyGamepadButton::CRABBY_GAMEPAD_NORTH as u32 == crate::PonkoGamepadButton::PONKO_GAMEPAD_NORTH as u32);
    assert!(CrabbyGamepadButton::CRABBY_GAMEPAD_WEST as u32 == crate::PonkoGamepadButton::PONKO_GAMEPAD_WEST as u32);
    assert!(
        CrabbyGamepadButton::CRABBY_GAMEPAD_LEFT_BUMPER as u32
            == crate::PonkoGamepadButton::PONKO_GAMEPAD_LEFT_BUMPER as u32
    );
    assert!(
        CrabbyGamepadButton::CRABBY_GAMEPAD_RIGHT_BUMPER as u32
            == crate::PonkoGamepadButton::PONKO_GAMEPAD_RIGHT_BUMPER as u32
    );
    assert!(
        CrabbyGamepadButton::CRABBY_GAMEPAD_LEFT_TRIGGER as u32
            == crate::PonkoGamepadButton::PONKO_GAMEPAD_LEFT_TRIGGER as u32
    );
    assert!(
        CrabbyGamepadButton::CRABBY_GAMEPAD_RIGHT_TRIGGER as u32
            == crate::PonkoGamepadButton::PONKO_GAMEPAD_RIGHT_TRIGGER as u32
    );
    assert!(
        CrabbyGamepadButton::CRABBY_GAMEPAD_SELECT as u32 == crate::PonkoGamepadButton::PONKO_GAMEPAD_SELECT as u32
    );
    assert!(CrabbyGamepadButton::CRABBY_GAMEPAD_START as u32 == crate::PonkoGamepadButton::PONKO_GAMEPAD_START as u32);
    assert!(
        CrabbyGamepadButton::CRABBY_GAMEPAD_LEFT_STICK as u32
            == crate::PonkoGamepadButton::PONKO_GAMEPAD_LEFT_STICK as u32
    );
    assert!(
        CrabbyGamepadButton::CRABBY_GAMEPAD_RIGHT_STICK as u32
            == crate::PonkoGamepadButton::PONKO_GAMEPAD_RIGHT_STICK as u32
    );
    assert!(
        CrabbyGamepadButton::CRABBY_GAMEPAD_DPAD_UP as u32 == crate::PonkoGamepadButton::PONKO_GAMEPAD_DPAD_UP as u32
    );
    assert!(
        CrabbyGamepadButton::CRABBY_GAMEPAD_DPAD_DOWN as u32
            == crate::PonkoGamepadButton::PONKO_GAMEPAD_DPAD_DOWN as u32
    );
    assert!(
        CrabbyGamepadButton::CRABBY_GAMEPAD_DPAD_LEFT as u32
            == crate::PonkoGamepadButton::PONKO_GAMEPAD_DPAD_LEFT as u32
    );
    assert!(
        CrabbyGamepadButton::CRABBY_GAMEPAD_DPAD_RIGHT as u32
            == crate::PonkoGamepadButton::PONKO_GAMEPAD_DPAD_RIGHT as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGamepadButton::CRABBY_GAMEPAD_DPAD_DOWN;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGamepadButton::CRABBY_GAMEPAD_DPAD_LEFT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGamepadButton::CRABBY_GAMEPAD_DPAD_RIGHT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGamepadButton::CRABBY_GAMEPAD_DPAD_UP;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGamepadButton::CRABBY_GAMEPAD_EAST;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGamepadButton::CRABBY_GAMEPAD_LEFT_BUMPER;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGamepadButton::CRABBY_GAMEPAD_LEFT_STICK;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGamepadButton::CRABBY_GAMEPAD_LEFT_TRIGGER;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGamepadButton::CRABBY_GAMEPAD_NORTH;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGamepadButton::CRABBY_GAMEPAD_RIGHT_BUMPER;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGamepadButton::CRABBY_GAMEPAD_RIGHT_STICK;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGamepadButton::CRABBY_GAMEPAD_RIGHT_TRIGGER;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGamepadButton::CRABBY_GAMEPAD_SELECT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGamepadButton::CRABBY_GAMEPAD_SOUTH;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGamepadButton::CRABBY_GAMEPAD_START;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGamepadButton::CRABBY_GAMEPAD_WEST;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyGamepadAxis {
    CRABBY_GAMEPAD_LEFT_STICK_X,

    CRABBY_GAMEPAD_LEFT_STICK_Y,

    CRABBY_GAMEPAD_RIGHT_STICK_X,

    CRABBY_GAMEPAD_RIGHT_STICK_Y,

    CRABBY_GAMEPAD_LEFT_TRIGGER_AXIS,

    CRABBY_GAMEPAD_RIGHT_TRIGGER_AXIS,
}
impl From<crate::PonkoGamepadAxis> for CrabbyGamepadAxis {
    fn from(value: crate::PonkoGamepadAxis) -> Self {
        match value {
            crate::PonkoGamepadAxis::PONKO_GAMEPAD_LEFT_STICK_X => Self::CRABBY_GAMEPAD_LEFT_STICK_X,
            crate::PonkoGamepadAxis::PONKO_GAMEPAD_LEFT_STICK_Y => Self::CRABBY_GAMEPAD_LEFT_STICK_Y,
            crate::PonkoGamepadAxis::PONKO_GAMEPAD_RIGHT_STICK_X => Self::CRABBY_GAMEPAD_RIGHT_STICK_X,
            crate::PonkoGamepadAxis::PONKO_GAMEPAD_RIGHT_STICK_Y => Self::CRABBY_GAMEPAD_RIGHT_STICK_Y,
            crate::PonkoGamepadAxis::PONKO_GAMEPAD_LEFT_TRIGGER_AXIS => Self::CRABBY_GAMEPAD_LEFT_TRIGGER_AXIS,
            crate::PonkoGamepadAxis::PONKO_GAMEPAD_RIGHT_TRIGGER_AXIS => Self::CRABBY_GAMEPAD_RIGHT_TRIGGER_AXIS,
        }
    }
}
impl From<CrabbyGamepadAxis> for crate::PonkoGamepadAxis {
    fn from(value: CrabbyGamepadAxis) -> Self {
        match value {
            CrabbyGamepadAxis::CRABBY_GAMEPAD_LEFT_STICK_X => Self::PONKO_GAMEPAD_LEFT_STICK_X,
            CrabbyGamepadAxis::CRABBY_GAMEPAD_LEFT_STICK_Y => Self::PONKO_GAMEPAD_LEFT_STICK_Y,
            CrabbyGamepadAxis::CRABBY_GAMEPAD_RIGHT_STICK_X => Self::PONKO_GAMEPAD_RIGHT_STICK_X,
            CrabbyGamepadAxis::CRABBY_GAMEPAD_RIGHT_STICK_Y => Self::PONKO_GAMEPAD_RIGHT_STICK_Y,
            CrabbyGamepadAxis::CRABBY_GAMEPAD_LEFT_TRIGGER_AXIS => Self::PONKO_GAMEPAD_LEFT_TRIGGER_AXIS,
            CrabbyGamepadAxis::CRABBY_GAMEPAD_RIGHT_TRIGGER_AXIS => Self::PONKO_GAMEPAD_RIGHT_TRIGGER_AXIS,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyGamepadAxis>() == std::mem::size_of::<crate::PonkoGamepadAxis>());
    assert!(std::mem::align_of::<CrabbyGamepadAxis>() == std::mem::align_of::<crate::PonkoGamepadAxis>());
    assert!(
        CrabbyGamepadAxis::CRABBY_GAMEPAD_LEFT_STICK_X as u32
            == crate::PonkoGamepadAxis::PONKO_GAMEPAD_LEFT_STICK_X as u32
    );
    assert!(
        CrabbyGamepadAxis::CRABBY_GAMEPAD_LEFT_STICK_Y as u32
            == crate::PonkoGamepadAxis::PONKO_GAMEPAD_LEFT_STICK_Y as u32
    );
    assert!(
        CrabbyGamepadAxis::CRABBY_GAMEPAD_RIGHT_STICK_X as u32
            == crate::PonkoGamepadAxis::PONKO_GAMEPAD_RIGHT_STICK_X as u32
    );
    assert!(
        CrabbyGamepadAxis::CRABBY_GAMEPAD_RIGHT_STICK_Y as u32
            == crate::PonkoGamepadAxis::PONKO_GAMEPAD_RIGHT_STICK_Y as u32
    );
    assert!(
        CrabbyGamepadAxis::CRABBY_GAMEPAD_LEFT_TRIGGER_AXIS as u32
            == crate::PonkoGamepadAxis::PONKO_GAMEPAD_LEFT_TRIGGER_AXIS as u32
    );
    assert!(
        CrabbyGamepadAxis::CRABBY_GAMEPAD_RIGHT_TRIGGER_AXIS as u32
            == crate::PonkoGamepadAxis::PONKO_GAMEPAD_RIGHT_TRIGGER_AXIS as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGamepadAxis::CRABBY_GAMEPAD_LEFT_STICK_X;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGamepadAxis::CRABBY_GAMEPAD_LEFT_STICK_Y;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGamepadAxis::CRABBY_GAMEPAD_LEFT_TRIGGER_AXIS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGamepadAxis::CRABBY_GAMEPAD_RIGHT_STICK_X;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGamepadAxis::CRABBY_GAMEPAD_RIGHT_STICK_Y;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyGamepadAxis::CRABBY_GAMEPAD_RIGHT_TRIGGER_AXIS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyStatus {
    CRABBY_OK = 0,

    CRABBY_INVALID_ARGUMENT = 1,

    CRABBY_NOT_FOUND = 2,

    CRABBY_BUSY = 3,

    CRABBY_PANIC = 4,
}
impl From<crate::PonkoStatus> for CrabbyStatus {
    fn from(value: crate::PonkoStatus) -> Self {
        match value {
            crate::PonkoStatus::PONKO_OK => Self::CRABBY_OK,
            crate::PonkoStatus::PONKO_INVALID_ARGUMENT => Self::CRABBY_INVALID_ARGUMENT,
            crate::PonkoStatus::PONKO_NOT_FOUND => Self::CRABBY_NOT_FOUND,
            crate::PonkoStatus::PONKO_BUSY => Self::CRABBY_BUSY,
            crate::PonkoStatus::PONKO_PANIC => Self::CRABBY_PANIC,
        }
    }
}
impl From<CrabbyStatus> for crate::PonkoStatus {
    fn from(value: CrabbyStatus) -> Self {
        match value {
            CrabbyStatus::CRABBY_OK => Self::PONKO_OK,
            CrabbyStatus::CRABBY_INVALID_ARGUMENT => Self::PONKO_INVALID_ARGUMENT,
            CrabbyStatus::CRABBY_NOT_FOUND => Self::PONKO_NOT_FOUND,
            CrabbyStatus::CRABBY_BUSY => Self::PONKO_BUSY,
            CrabbyStatus::CRABBY_PANIC => Self::PONKO_PANIC,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyStatus>() == std::mem::size_of::<crate::PonkoStatus>());
    assert!(std::mem::align_of::<CrabbyStatus>() == std::mem::align_of::<crate::PonkoStatus>());
    assert!(CrabbyStatus::CRABBY_OK as u32 == crate::PonkoStatus::PONKO_OK as u32);
    assert!(CrabbyStatus::CRABBY_INVALID_ARGUMENT as u32 == crate::PonkoStatus::PONKO_INVALID_ARGUMENT as u32);
    assert!(CrabbyStatus::CRABBY_NOT_FOUND as u32 == crate::PonkoStatus::PONKO_NOT_FOUND as u32);
    assert!(CrabbyStatus::CRABBY_BUSY as u32 == crate::PonkoStatus::PONKO_BUSY as u32);
    assert!(CrabbyStatus::CRABBY_PANIC as u32 == crate::PonkoStatus::PONKO_PANIC as u32);
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoChunk as CrabbyChunk;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoQueryFn as CrabbyQueryFn;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoValue as CrabbyValue;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoWorld as CrabbyWorld;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyStatus::CRABBY_BUSY;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyStatus::CRABBY_INVALID_ARGUMENT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyStatus::CRABBY_NOT_FOUND;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyStatus::CRABBY_OK;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyStatus::CRABBY_PANIC;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyErrorKind {
    CRABBY_ERROR_NONE = 0,

    CRABBY_ERROR_NOT_FOUND = 1,

    CRABBY_ERROR_IO = 2,

    CRABBY_ERROR_DECODE = 3,

    CRABBY_ERROR_SHADER = 4,

    CRABBY_ERROR_UNSUPPORTED = 5,

    CRABBY_ERROR_BAD_HANDLE = 6,

    CRABBY_ERROR_JSON = 7,

    CRABBY_ERROR_LIMIT = 8,

    CRABBY_ERROR_INVALID = 9,

    CRABBY_ERROR_OTHER = 10,
}
impl From<crate::PonkoErrorKind> for CrabbyErrorKind {
    fn from(value: crate::PonkoErrorKind) -> Self {
        match value {
            crate::PonkoErrorKind::PONKO_ERROR_NONE => Self::CRABBY_ERROR_NONE,
            crate::PonkoErrorKind::PONKO_ERROR_NOT_FOUND => Self::CRABBY_ERROR_NOT_FOUND,
            crate::PonkoErrorKind::PONKO_ERROR_IO => Self::CRABBY_ERROR_IO,
            crate::PonkoErrorKind::PONKO_ERROR_DECODE => Self::CRABBY_ERROR_DECODE,
            crate::PonkoErrorKind::PONKO_ERROR_SHADER => Self::CRABBY_ERROR_SHADER,
            crate::PonkoErrorKind::PONKO_ERROR_UNSUPPORTED => Self::CRABBY_ERROR_UNSUPPORTED,
            crate::PonkoErrorKind::PONKO_ERROR_BAD_HANDLE => Self::CRABBY_ERROR_BAD_HANDLE,
            crate::PonkoErrorKind::PONKO_ERROR_JSON => Self::CRABBY_ERROR_JSON,
            crate::PonkoErrorKind::PONKO_ERROR_LIMIT => Self::CRABBY_ERROR_LIMIT,
            crate::PonkoErrorKind::PONKO_ERROR_INVALID => Self::CRABBY_ERROR_INVALID,
            crate::PonkoErrorKind::PONKO_ERROR_OTHER => Self::CRABBY_ERROR_OTHER,
        }
    }
}
impl From<CrabbyErrorKind> for crate::PonkoErrorKind {
    fn from(value: CrabbyErrorKind) -> Self {
        match value {
            CrabbyErrorKind::CRABBY_ERROR_NONE => Self::PONKO_ERROR_NONE,
            CrabbyErrorKind::CRABBY_ERROR_NOT_FOUND => Self::PONKO_ERROR_NOT_FOUND,
            CrabbyErrorKind::CRABBY_ERROR_IO => Self::PONKO_ERROR_IO,
            CrabbyErrorKind::CRABBY_ERROR_DECODE => Self::PONKO_ERROR_DECODE,
            CrabbyErrorKind::CRABBY_ERROR_SHADER => Self::PONKO_ERROR_SHADER,
            CrabbyErrorKind::CRABBY_ERROR_UNSUPPORTED => Self::PONKO_ERROR_UNSUPPORTED,
            CrabbyErrorKind::CRABBY_ERROR_BAD_HANDLE => Self::PONKO_ERROR_BAD_HANDLE,
            CrabbyErrorKind::CRABBY_ERROR_JSON => Self::PONKO_ERROR_JSON,
            CrabbyErrorKind::CRABBY_ERROR_LIMIT => Self::PONKO_ERROR_LIMIT,
            CrabbyErrorKind::CRABBY_ERROR_INVALID => Self::PONKO_ERROR_INVALID,
            CrabbyErrorKind::CRABBY_ERROR_OTHER => Self::PONKO_ERROR_OTHER,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyErrorKind>() == std::mem::size_of::<crate::PonkoErrorKind>());
    assert!(std::mem::align_of::<CrabbyErrorKind>() == std::mem::align_of::<crate::PonkoErrorKind>());
    assert!(CrabbyErrorKind::CRABBY_ERROR_NONE as u32 == crate::PonkoErrorKind::PONKO_ERROR_NONE as u32);
    assert!(CrabbyErrorKind::CRABBY_ERROR_NOT_FOUND as u32 == crate::PonkoErrorKind::PONKO_ERROR_NOT_FOUND as u32);
    assert!(CrabbyErrorKind::CRABBY_ERROR_IO as u32 == crate::PonkoErrorKind::PONKO_ERROR_IO as u32);
    assert!(CrabbyErrorKind::CRABBY_ERROR_DECODE as u32 == crate::PonkoErrorKind::PONKO_ERROR_DECODE as u32);
    assert!(CrabbyErrorKind::CRABBY_ERROR_SHADER as u32 == crate::PonkoErrorKind::PONKO_ERROR_SHADER as u32);
    assert!(CrabbyErrorKind::CRABBY_ERROR_UNSUPPORTED as u32 == crate::PonkoErrorKind::PONKO_ERROR_UNSUPPORTED as u32);
    assert!(CrabbyErrorKind::CRABBY_ERROR_BAD_HANDLE as u32 == crate::PonkoErrorKind::PONKO_ERROR_BAD_HANDLE as u32);
    assert!(CrabbyErrorKind::CRABBY_ERROR_JSON as u32 == crate::PonkoErrorKind::PONKO_ERROR_JSON as u32);
    assert!(CrabbyErrorKind::CRABBY_ERROR_LIMIT as u32 == crate::PonkoErrorKind::PONKO_ERROR_LIMIT as u32);
    assert!(CrabbyErrorKind::CRABBY_ERROR_INVALID as u32 == crate::PonkoErrorKind::PONKO_ERROR_INVALID as u32);
    assert!(CrabbyErrorKind::CRABBY_ERROR_OTHER as u32 == crate::PonkoErrorKind::PONKO_ERROR_OTHER as u32);
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoFont as CrabbyFont;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoNetMessages as CrabbyNetMessages;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoPlaying as CrabbyPlaying;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoSound as CrabbySound;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoSoundParams as CrabbySoundParams;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoTextParams as CrabbyTextParams;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoTurns as CrabbyTurns;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoUiStyle as CrabbyUiStyle;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyErrorKind::CRABBY_ERROR_BAD_HANDLE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyErrorKind::CRABBY_ERROR_DECODE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyErrorKind::CRABBY_ERROR_INVALID;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyErrorKind::CRABBY_ERROR_IO;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyErrorKind::CRABBY_ERROR_JSON;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyErrorKind::CRABBY_ERROR_LIMIT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyErrorKind::CRABBY_ERROR_NONE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyErrorKind::CRABBY_ERROR_NOT_FOUND;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyErrorKind::CRABBY_ERROR_OTHER;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyErrorKind::CRABBY_ERROR_SHADER;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyErrorKind::CRABBY_ERROR_UNSUPPORTED;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyLane {
    CRABBY_LANE_RELIABLE = 0,

    CRABBY_LANE_UNRELIABLE = 1,

    CRABBY_LANE_RELIABLE_UNORDERED = 2,
}
impl From<crate::PonkoLane> for CrabbyLane {
    fn from(value: crate::PonkoLane) -> Self {
        match value {
            crate::PonkoLane::PONKO_LANE_RELIABLE => Self::CRABBY_LANE_RELIABLE,
            crate::PonkoLane::PONKO_LANE_UNRELIABLE => Self::CRABBY_LANE_UNRELIABLE,
            crate::PonkoLane::PONKO_LANE_RELIABLE_UNORDERED => Self::CRABBY_LANE_RELIABLE_UNORDERED,
        }
    }
}
impl From<CrabbyLane> for crate::PonkoLane {
    fn from(value: CrabbyLane) -> Self {
        match value {
            CrabbyLane::CRABBY_LANE_RELIABLE => Self::PONKO_LANE_RELIABLE,
            CrabbyLane::CRABBY_LANE_UNRELIABLE => Self::PONKO_LANE_UNRELIABLE,
            CrabbyLane::CRABBY_LANE_RELIABLE_UNORDERED => Self::PONKO_LANE_RELIABLE_UNORDERED,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyLane>() == std::mem::size_of::<crate::PonkoLane>());
    assert!(std::mem::align_of::<CrabbyLane>() == std::mem::align_of::<crate::PonkoLane>());
    assert!(CrabbyLane::CRABBY_LANE_RELIABLE as u32 == crate::PonkoLane::PONKO_LANE_RELIABLE as u32);
    assert!(CrabbyLane::CRABBY_LANE_UNRELIABLE as u32 == crate::PonkoLane::PONKO_LANE_UNRELIABLE as u32);
    assert!(
        CrabbyLane::CRABBY_LANE_RELIABLE_UNORDERED as u32 == crate::PonkoLane::PONKO_LANE_RELIABLE_UNORDERED as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyLane::CRABBY_LANE_RELIABLE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyLane::CRABBY_LANE_RELIABLE_UNORDERED;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyLane::CRABBY_LANE_UNRELIABLE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyMessageEventKind {
    CRABBY_MESSAGE_CONNECTED = 0,

    CRABBY_MESSAGE_DISCONNECTED = 1,

    CRABBY_MESSAGE = 2,
}
impl From<crate::PonkoMessageEventKind> for CrabbyMessageEventKind {
    fn from(value: crate::PonkoMessageEventKind) -> Self {
        match value {
            crate::PonkoMessageEventKind::PONKO_MESSAGE_CONNECTED => Self::CRABBY_MESSAGE_CONNECTED,
            crate::PonkoMessageEventKind::PONKO_MESSAGE_DISCONNECTED => Self::CRABBY_MESSAGE_DISCONNECTED,
            crate::PonkoMessageEventKind::PONKO_MESSAGE => Self::CRABBY_MESSAGE,
        }
    }
}
impl From<CrabbyMessageEventKind> for crate::PonkoMessageEventKind {
    fn from(value: CrabbyMessageEventKind) -> Self {
        match value {
            CrabbyMessageEventKind::CRABBY_MESSAGE_CONNECTED => Self::PONKO_MESSAGE_CONNECTED,
            CrabbyMessageEventKind::CRABBY_MESSAGE_DISCONNECTED => Self::PONKO_MESSAGE_DISCONNECTED,
            CrabbyMessageEventKind::CRABBY_MESSAGE => Self::PONKO_MESSAGE,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyMessageEventKind>() == std::mem::size_of::<crate::PonkoMessageEventKind>());
    assert!(std::mem::align_of::<CrabbyMessageEventKind>() == std::mem::align_of::<crate::PonkoMessageEventKind>());
    assert!(
        CrabbyMessageEventKind::CRABBY_MESSAGE_CONNECTED as u32
            == crate::PonkoMessageEventKind::PONKO_MESSAGE_CONNECTED as u32
    );
    assert!(
        CrabbyMessageEventKind::CRABBY_MESSAGE_DISCONNECTED as u32
            == crate::PonkoMessageEventKind::PONKO_MESSAGE_DISCONNECTED as u32
    );
    assert!(CrabbyMessageEventKind::CRABBY_MESSAGE as u32 == crate::PonkoMessageEventKind::PONKO_MESSAGE as u32);
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyMessageEventKind::CRABBY_MESSAGE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyMessageEventKind::CRABBY_MESSAGE_CONNECTED;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyMessageEventKind::CRABBY_MESSAGE_DISCONNECTED;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CrabbyMessageEvent {
    pub kind: CrabbyMessageEventKind,

    pub peer: u64,

    pub lane: CrabbyLane,

    pub bytes: *const u8,

    pub len: usize,

    pub reason: *const c_char,
}
impl From<crate::PonkoMessageEvent> for CrabbyMessageEvent {
    fn from(value: crate::PonkoMessageEvent) -> Self {
        Self {
            kind: value.kind.into(),
            peer: value.peer,
            lane: value.lane.into(),
            bytes: value.bytes,
            len: value.len,
            reason: value.reason,
        }
    }
}
impl From<CrabbyMessageEvent> for crate::PonkoMessageEvent {
    fn from(value: CrabbyMessageEvent) -> Self {
        Self {
            kind: value.kind.into(),
            peer: value.peer,
            lane: value.lane.into(),
            bytes: value.bytes,
            len: value.len,
            reason: value.reason,
        }
    }
}
impl Default for CrabbyMessageEvent {
    fn default() -> Self {
        crate::PonkoMessageEvent::default().into()
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyMessageEvent>() == std::mem::size_of::<crate::PonkoMessageEvent>());
    assert!(std::mem::align_of::<CrabbyMessageEvent>() == std::mem::align_of::<crate::PonkoMessageEvent>());
    assert!(std::mem::offset_of!(CrabbyMessageEvent, kind) == std::mem::offset_of!(crate::PonkoMessageEvent, kind));
    assert!(std::mem::offset_of!(CrabbyMessageEvent, peer) == std::mem::offset_of!(crate::PonkoMessageEvent, peer));
    assert!(std::mem::offset_of!(CrabbyMessageEvent, lane) == std::mem::offset_of!(crate::PonkoMessageEvent, lane));
    assert!(std::mem::offset_of!(CrabbyMessageEvent, bytes) == std::mem::offset_of!(crate::PonkoMessageEvent, bytes));
    assert!(std::mem::offset_of!(CrabbyMessageEvent, len) == std::mem::offset_of!(crate::PonkoMessageEvent, len));
    assert!(std::mem::offset_of!(CrabbyMessageEvent, reason) == std::mem::offset_of!(crate::PonkoMessageEvent, reason));
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoMessageStats as CrabbyMessageStats;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoNavBakeSettings as CrabbyNavBakeSettings;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoNavSources as CrabbyNavSources;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyNetState {
    CRABBY_NET_OFFLINE,

    CRABBY_NET_CONNECTING,

    CRABBY_NET_RUNNING,

    CRABBY_NET_LOST,
}
impl From<crate::PonkoNetState> for CrabbyNetState {
    fn from(value: crate::PonkoNetState) -> Self {
        match value {
            crate::PonkoNetState::PONKO_NET_OFFLINE => Self::CRABBY_NET_OFFLINE,
            crate::PonkoNetState::PONKO_NET_CONNECTING => Self::CRABBY_NET_CONNECTING,
            crate::PonkoNetState::PONKO_NET_RUNNING => Self::CRABBY_NET_RUNNING,
            crate::PonkoNetState::PONKO_NET_LOST => Self::CRABBY_NET_LOST,
        }
    }
}
impl From<CrabbyNetState> for crate::PonkoNetState {
    fn from(value: CrabbyNetState) -> Self {
        match value {
            CrabbyNetState::CRABBY_NET_OFFLINE => Self::PONKO_NET_OFFLINE,
            CrabbyNetState::CRABBY_NET_CONNECTING => Self::PONKO_NET_CONNECTING,
            CrabbyNetState::CRABBY_NET_RUNNING => Self::PONKO_NET_RUNNING,
            CrabbyNetState::CRABBY_NET_LOST => Self::PONKO_NET_LOST,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyNetState>() == std::mem::size_of::<crate::PonkoNetState>());
    assert!(std::mem::align_of::<CrabbyNetState>() == std::mem::align_of::<crate::PonkoNetState>());
    assert!(CrabbyNetState::CRABBY_NET_OFFLINE as u32 == crate::PonkoNetState::PONKO_NET_OFFLINE as u32);
    assert!(CrabbyNetState::CRABBY_NET_CONNECTING as u32 == crate::PonkoNetState::PONKO_NET_CONNECTING as u32);
    assert!(CrabbyNetState::CRABBY_NET_RUNNING as u32 == crate::PonkoNetState::PONKO_NET_RUNNING as u32);
    assert!(CrabbyNetState::CRABBY_NET_LOST as u32 == crate::PonkoNetState::PONKO_NET_LOST as u32);
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyNetState::CRABBY_NET_CONNECTING;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyNetState::CRABBY_NET_LOST;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyNetState::CRABBY_NET_OFFLINE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyNetState::CRABBY_NET_RUNNING;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyParticleShape3D {
    CRABBY_PARTICLES_POINT,

    CRABBY_PARTICLES_SPHERE,

    CRABBY_PARTICLES_CONE,

    CRABBY_PARTICLES_BOX,
}
impl From<crate::PonkoParticleShape3D> for CrabbyParticleShape3D {
    fn from(value: crate::PonkoParticleShape3D) -> Self {
        match value {
            crate::PonkoParticleShape3D::PONKO_PARTICLES_POINT => Self::CRABBY_PARTICLES_POINT,
            crate::PonkoParticleShape3D::PONKO_PARTICLES_SPHERE => Self::CRABBY_PARTICLES_SPHERE,
            crate::PonkoParticleShape3D::PONKO_PARTICLES_CONE => Self::CRABBY_PARTICLES_CONE,
            crate::PonkoParticleShape3D::PONKO_PARTICLES_BOX => Self::CRABBY_PARTICLES_BOX,
        }
    }
}
impl From<CrabbyParticleShape3D> for crate::PonkoParticleShape3D {
    fn from(value: CrabbyParticleShape3D) -> Self {
        match value {
            CrabbyParticleShape3D::CRABBY_PARTICLES_POINT => Self::PONKO_PARTICLES_POINT,
            CrabbyParticleShape3D::CRABBY_PARTICLES_SPHERE => Self::PONKO_PARTICLES_SPHERE,
            CrabbyParticleShape3D::CRABBY_PARTICLES_CONE => Self::PONKO_PARTICLES_CONE,
            CrabbyParticleShape3D::CRABBY_PARTICLES_BOX => Self::PONKO_PARTICLES_BOX,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyParticleShape3D>() == std::mem::size_of::<crate::PonkoParticleShape3D>());
    assert!(std::mem::align_of::<CrabbyParticleShape3D>() == std::mem::align_of::<crate::PonkoParticleShape3D>());
    assert!(
        CrabbyParticleShape3D::CRABBY_PARTICLES_POINT as u32
            == crate::PonkoParticleShape3D::PONKO_PARTICLES_POINT as u32
    );
    assert!(
        CrabbyParticleShape3D::CRABBY_PARTICLES_SPHERE as u32
            == crate::PonkoParticleShape3D::PONKO_PARTICLES_SPHERE as u32
    );
    assert!(
        CrabbyParticleShape3D::CRABBY_PARTICLES_CONE as u32 == crate::PonkoParticleShape3D::PONKO_PARTICLES_CONE as u32
    );
    assert!(
        CrabbyParticleShape3D::CRABBY_PARTICLES_BOX as u32 == crate::PonkoParticleShape3D::PONKO_PARTICLES_BOX as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyParticleShape3D::CRABBY_PARTICLES_BOX;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyParticleShape3D::CRABBY_PARTICLES_CONE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyParticleShape3D::CRABBY_PARTICLES_POINT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyParticleShape3D::CRABBY_PARTICLES_SPHERE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyParticleBlend3D {
    CRABBY_PARTICLES_ALPHA,

    CRABBY_PARTICLES_ADDITIVE,
}
impl From<crate::PonkoParticleBlend3D> for CrabbyParticleBlend3D {
    fn from(value: crate::PonkoParticleBlend3D) -> Self {
        match value {
            crate::PonkoParticleBlend3D::PONKO_PARTICLES_ALPHA => Self::CRABBY_PARTICLES_ALPHA,
            crate::PonkoParticleBlend3D::PONKO_PARTICLES_ADDITIVE => Self::CRABBY_PARTICLES_ADDITIVE,
        }
    }
}
impl From<CrabbyParticleBlend3D> for crate::PonkoParticleBlend3D {
    fn from(value: CrabbyParticleBlend3D) -> Self {
        match value {
            CrabbyParticleBlend3D::CRABBY_PARTICLES_ALPHA => Self::PONKO_PARTICLES_ALPHA,
            CrabbyParticleBlend3D::CRABBY_PARTICLES_ADDITIVE => Self::PONKO_PARTICLES_ADDITIVE,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyParticleBlend3D>() == std::mem::size_of::<crate::PonkoParticleBlend3D>());
    assert!(std::mem::align_of::<CrabbyParticleBlend3D>() == std::mem::align_of::<crate::PonkoParticleBlend3D>());
    assert!(
        CrabbyParticleBlend3D::CRABBY_PARTICLES_ALPHA as u32
            == crate::PonkoParticleBlend3D::PONKO_PARTICLES_ALPHA as u32
    );
    assert!(
        CrabbyParticleBlend3D::CRABBY_PARTICLES_ADDITIVE as u32
            == crate::PonkoParticleBlend3D::PONKO_PARTICLES_ADDITIVE as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyParticleBlend3D::CRABBY_PARTICLES_ADDITIVE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyParticleBlend3D::CRABBY_PARTICLES_ALPHA;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyParticleSpace3D {
    CRABBY_PARTICLES_WORLD,

    CRABBY_PARTICLES_LOCAL,
}
impl From<crate::PonkoParticleSpace3D> for CrabbyParticleSpace3D {
    fn from(value: crate::PonkoParticleSpace3D) -> Self {
        match value {
            crate::PonkoParticleSpace3D::PONKO_PARTICLES_WORLD => Self::CRABBY_PARTICLES_WORLD,
            crate::PonkoParticleSpace3D::PONKO_PARTICLES_LOCAL => Self::CRABBY_PARTICLES_LOCAL,
        }
    }
}
impl From<CrabbyParticleSpace3D> for crate::PonkoParticleSpace3D {
    fn from(value: CrabbyParticleSpace3D) -> Self {
        match value {
            CrabbyParticleSpace3D::CRABBY_PARTICLES_WORLD => Self::PONKO_PARTICLES_WORLD,
            CrabbyParticleSpace3D::CRABBY_PARTICLES_LOCAL => Self::PONKO_PARTICLES_LOCAL,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyParticleSpace3D>() == std::mem::size_of::<crate::PonkoParticleSpace3D>());
    assert!(std::mem::align_of::<CrabbyParticleSpace3D>() == std::mem::align_of::<crate::PonkoParticleSpace3D>());
    assert!(
        CrabbyParticleSpace3D::CRABBY_PARTICLES_WORLD as u32
            == crate::PonkoParticleSpace3D::PONKO_PARTICLES_WORLD as u32
    );
    assert!(
        CrabbyParticleSpace3D::CRABBY_PARTICLES_LOCAL as u32
            == crate::PonkoParticleSpace3D::PONKO_PARTICLES_LOCAL as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoFrameStats as CrabbyFrameStats;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoGpuUploads as CrabbyGpuUploads;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoParticleColorKey as CrabbyParticleColorKey;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoParticleKey as CrabbyParticleKey;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoParticleSettings3D as CrabbyParticleSettings3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoProfileScope as CrabbyProfileScope;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyParticleSpace3D::CRABBY_PARTICLES_LOCAL;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyParticleSpace3D::CRABBY_PARTICLES_WORLD;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyBodyKind {
    CRABBY_BODY_DYNAMIC,

    CRABBY_BODY_FIXED,

    CRABBY_BODY_KINEMATIC,
}
impl From<crate::PonkoBodyKind> for CrabbyBodyKind {
    fn from(value: crate::PonkoBodyKind) -> Self {
        match value {
            crate::PonkoBodyKind::PONKO_BODY_DYNAMIC => Self::CRABBY_BODY_DYNAMIC,
            crate::PonkoBodyKind::PONKO_BODY_FIXED => Self::CRABBY_BODY_FIXED,
            crate::PonkoBodyKind::PONKO_BODY_KINEMATIC => Self::CRABBY_BODY_KINEMATIC,
        }
    }
}
impl From<CrabbyBodyKind> for crate::PonkoBodyKind {
    fn from(value: CrabbyBodyKind) -> Self {
        match value {
            CrabbyBodyKind::CRABBY_BODY_DYNAMIC => Self::PONKO_BODY_DYNAMIC,
            CrabbyBodyKind::CRABBY_BODY_FIXED => Self::PONKO_BODY_FIXED,
            CrabbyBodyKind::CRABBY_BODY_KINEMATIC => Self::PONKO_BODY_KINEMATIC,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyBodyKind>() == std::mem::size_of::<crate::PonkoBodyKind>());
    assert!(std::mem::align_of::<CrabbyBodyKind>() == std::mem::align_of::<crate::PonkoBodyKind>());
    assert!(CrabbyBodyKind::CRABBY_BODY_DYNAMIC as u32 == crate::PonkoBodyKind::PONKO_BODY_DYNAMIC as u32);
    assert!(CrabbyBodyKind::CRABBY_BODY_FIXED as u32 == crate::PonkoBodyKind::PONKO_BODY_FIXED as u32);
    assert!(CrabbyBodyKind::CRABBY_BODY_KINEMATIC as u32 == crate::PonkoBodyKind::PONKO_BODY_KINEMATIC as u32);
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyBodyKind::CRABBY_BODY_DYNAMIC;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyBodyKind::CRABBY_BODY_FIXED;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyBodyKind::CRABBY_BODY_KINEMATIC;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyShapeKind {
    CRABBY_SHAPE_CIRCLE,

    CRABBY_SHAPE_RECT,

    CRABBY_SHAPE_CAPSULE,

    CRABBY_SHAPE_POLYGON,
}
impl From<crate::PonkoShapeKind> for CrabbyShapeKind {
    fn from(value: crate::PonkoShapeKind) -> Self {
        match value {
            crate::PonkoShapeKind::PONKO_SHAPE_CIRCLE => Self::CRABBY_SHAPE_CIRCLE,
            crate::PonkoShapeKind::PONKO_SHAPE_RECT => Self::CRABBY_SHAPE_RECT,
            crate::PonkoShapeKind::PONKO_SHAPE_CAPSULE => Self::CRABBY_SHAPE_CAPSULE,
            crate::PonkoShapeKind::PONKO_SHAPE_POLYGON => Self::CRABBY_SHAPE_POLYGON,
        }
    }
}
impl From<CrabbyShapeKind> for crate::PonkoShapeKind {
    fn from(value: CrabbyShapeKind) -> Self {
        match value {
            CrabbyShapeKind::CRABBY_SHAPE_CIRCLE => Self::PONKO_SHAPE_CIRCLE,
            CrabbyShapeKind::CRABBY_SHAPE_RECT => Self::PONKO_SHAPE_RECT,
            CrabbyShapeKind::CRABBY_SHAPE_CAPSULE => Self::PONKO_SHAPE_CAPSULE,
            CrabbyShapeKind::CRABBY_SHAPE_POLYGON => Self::PONKO_SHAPE_POLYGON,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyShapeKind>() == std::mem::size_of::<crate::PonkoShapeKind>());
    assert!(std::mem::align_of::<CrabbyShapeKind>() == std::mem::align_of::<crate::PonkoShapeKind>());
    assert!(CrabbyShapeKind::CRABBY_SHAPE_CIRCLE as u32 == crate::PonkoShapeKind::PONKO_SHAPE_CIRCLE as u32);
    assert!(CrabbyShapeKind::CRABBY_SHAPE_RECT as u32 == crate::PonkoShapeKind::PONKO_SHAPE_RECT as u32);
    assert!(CrabbyShapeKind::CRABBY_SHAPE_CAPSULE as u32 == crate::PonkoShapeKind::PONKO_SHAPE_CAPSULE as u32);
    assert!(CrabbyShapeKind::CRABBY_SHAPE_POLYGON as u32 == crate::PonkoShapeKind::PONKO_SHAPE_POLYGON as u32);
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoCollider as CrabbyCollider;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoCollision as CrabbyCollision;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoRayHit as CrabbyRayHit;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoRigidBody as CrabbyRigidBody;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoVelocity as CrabbyVelocity;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyShapeKind::CRABBY_SHAPE_CAPSULE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyShapeKind::CRABBY_SHAPE_CIRCLE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyShapeKind::CRABBY_SHAPE_POLYGON;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyShapeKind::CRABBY_SHAPE_RECT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyShape3DKind {
    CRABBY_SHAPE3D_SPHERE,

    CRABBY_SHAPE3D_BOX,

    CRABBY_SHAPE3D_CAPSULE,

    CRABBY_SHAPE3D_MESH,
}
impl From<crate::PonkoShape3DKind> for CrabbyShape3DKind {
    fn from(value: crate::PonkoShape3DKind) -> Self {
        match value {
            crate::PonkoShape3DKind::PONKO_SHAPE3D_SPHERE => Self::CRABBY_SHAPE3D_SPHERE,
            crate::PonkoShape3DKind::PONKO_SHAPE3D_BOX => Self::CRABBY_SHAPE3D_BOX,
            crate::PonkoShape3DKind::PONKO_SHAPE3D_CAPSULE => Self::CRABBY_SHAPE3D_CAPSULE,
            crate::PonkoShape3DKind::PONKO_SHAPE3D_MESH => Self::CRABBY_SHAPE3D_MESH,
        }
    }
}
impl From<CrabbyShape3DKind> for crate::PonkoShape3DKind {
    fn from(value: CrabbyShape3DKind) -> Self {
        match value {
            CrabbyShape3DKind::CRABBY_SHAPE3D_SPHERE => Self::PONKO_SHAPE3D_SPHERE,
            CrabbyShape3DKind::CRABBY_SHAPE3D_BOX => Self::PONKO_SHAPE3D_BOX,
            CrabbyShape3DKind::CRABBY_SHAPE3D_CAPSULE => Self::PONKO_SHAPE3D_CAPSULE,
            CrabbyShape3DKind::CRABBY_SHAPE3D_MESH => Self::PONKO_SHAPE3D_MESH,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyShape3DKind>() == std::mem::size_of::<crate::PonkoShape3DKind>());
    assert!(std::mem::align_of::<CrabbyShape3DKind>() == std::mem::align_of::<crate::PonkoShape3DKind>());
    assert!(CrabbyShape3DKind::CRABBY_SHAPE3D_SPHERE as u32 == crate::PonkoShape3DKind::PONKO_SHAPE3D_SPHERE as u32);
    assert!(CrabbyShape3DKind::CRABBY_SHAPE3D_BOX as u32 == crate::PonkoShape3DKind::PONKO_SHAPE3D_BOX as u32);
    assert!(CrabbyShape3DKind::CRABBY_SHAPE3D_CAPSULE as u32 == crate::PonkoShape3DKind::PONKO_SHAPE3D_CAPSULE as u32);
    assert!(CrabbyShape3DKind::CRABBY_SHAPE3D_MESH as u32 == crate::PonkoShape3DKind::PONKO_SHAPE3D_MESH as u32);
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoCollider3D as CrabbyCollider3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoCollisionGroups as CrabbyCollisionGroups;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoCollisionMesh as CrabbyCollisionMesh;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoRigidBody3D as CrabbyRigidBody3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoVelocity3D as CrabbyVelocity3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyShape3DKind::CRABBY_SHAPE3D_BOX;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyShape3DKind::CRABBY_SHAPE3D_CAPSULE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyShape3DKind::CRABBY_SHAPE3D_MESH;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyShape3DKind::CRABBY_SHAPE3D_SPHERE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyJointMotionKind {
    CRABBY_JOINT_FREE = 0,

    CRABBY_JOINT_LOCKED = 1,

    CRABBY_JOINT_LIMITED = 2,
}
impl From<crate::PonkoJointMotionKind> for CrabbyJointMotionKind {
    fn from(value: crate::PonkoJointMotionKind) -> Self {
        match value {
            crate::PonkoJointMotionKind::PONKO_JOINT_FREE => Self::CRABBY_JOINT_FREE,
            crate::PonkoJointMotionKind::PONKO_JOINT_LOCKED => Self::CRABBY_JOINT_LOCKED,
            crate::PonkoJointMotionKind::PONKO_JOINT_LIMITED => Self::CRABBY_JOINT_LIMITED,
        }
    }
}
impl From<CrabbyJointMotionKind> for crate::PonkoJointMotionKind {
    fn from(value: CrabbyJointMotionKind) -> Self {
        match value {
            CrabbyJointMotionKind::CRABBY_JOINT_FREE => Self::PONKO_JOINT_FREE,
            CrabbyJointMotionKind::CRABBY_JOINT_LOCKED => Self::PONKO_JOINT_LOCKED,
            CrabbyJointMotionKind::CRABBY_JOINT_LIMITED => Self::PONKO_JOINT_LIMITED,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyJointMotionKind>() == std::mem::size_of::<crate::PonkoJointMotionKind>());
    assert!(std::mem::align_of::<CrabbyJointMotionKind>() == std::mem::align_of::<crate::PonkoJointMotionKind>());
    assert!(CrabbyJointMotionKind::CRABBY_JOINT_FREE as u32 == crate::PonkoJointMotionKind::PONKO_JOINT_FREE as u32);
    assert!(
        CrabbyJointMotionKind::CRABBY_JOINT_LOCKED as u32 == crate::PonkoJointMotionKind::PONKO_JOINT_LOCKED as u32
    );
    assert!(
        CrabbyJointMotionKind::CRABBY_JOINT_LIMITED as u32 == crate::PonkoJointMotionKind::PONKO_JOINT_LIMITED as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyJointMotionKind::CRABBY_JOINT_FREE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyJointMotionKind::CRABBY_JOINT_LIMITED;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyJointMotionKind::CRABBY_JOINT_LOCKED;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyJointMotion3D {
    pub kind: CrabbyJointMotionKind,

    pub min: f32,

    pub max: f32,
}
impl From<crate::PonkoJointMotion3D> for CrabbyJointMotion3D {
    fn from(value: crate::PonkoJointMotion3D) -> Self {
        Self {
            kind: value.kind.into(),
            min: value.min,
            max: value.max,
        }
    }
}
impl From<CrabbyJointMotion3D> for crate::PonkoJointMotion3D {
    fn from(value: CrabbyJointMotion3D) -> Self {
        Self {
            kind: value.kind.into(),
            min: value.min,
            max: value.max,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyJointMotion3D>() == std::mem::size_of::<crate::PonkoJointMotion3D>());
    assert!(std::mem::align_of::<CrabbyJointMotion3D>() == std::mem::align_of::<crate::PonkoJointMotion3D>());
    assert!(std::mem::offset_of!(CrabbyJointMotion3D, kind) == std::mem::offset_of!(crate::PonkoJointMotion3D, kind));
    assert!(std::mem::offset_of!(CrabbyJointMotion3D, min) == std::mem::offset_of!(crate::PonkoJointMotion3D, min));
    assert!(std::mem::offset_of!(CrabbyJointMotion3D, max) == std::mem::offset_of!(crate::PonkoJointMotion3D, max));
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoJointDrive3D as CrabbyJointDrive3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyJoint3D {
    pub body_a: u64,

    pub body_b: u64,

    pub frame_a: CrabbyTransform3D,

    pub frame_b: CrabbyTransform3D,

    pub axes: [CrabbyJointMotion3D; 6],

    pub drives: [CrabbyJointDrive3D; 6],

    pub contacts_enabled: bool,

    pub swing_cone: f32,
}
impl From<crate::PonkoJoint3D> for CrabbyJoint3D {
    fn from(value: crate::PonkoJoint3D) -> Self {
        Self {
            body_a: value.body_a,
            body_b: value.body_b,
            frame_a: value.frame_a,
            frame_b: value.frame_b,
            axes: value.axes.map(|v| v.into()),
            drives: value.drives,
            contacts_enabled: value.contacts_enabled,
            swing_cone: value.swing_cone,
        }
    }
}
impl From<CrabbyJoint3D> for crate::PonkoJoint3D {
    fn from(value: CrabbyJoint3D) -> Self {
        Self {
            body_a: value.body_a,
            body_b: value.body_b,
            frame_a: value.frame_a,
            frame_b: value.frame_b,
            axes: value.axes.map(|v| v.into()),
            drives: value.drives,
            contacts_enabled: value.contacts_enabled,
            swing_cone: value.swing_cone,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyJoint3D>() == std::mem::size_of::<crate::PonkoJoint3D>());
    assert!(std::mem::align_of::<CrabbyJoint3D>() == std::mem::align_of::<crate::PonkoJoint3D>());
    assert!(std::mem::offset_of!(CrabbyJoint3D, body_a) == std::mem::offset_of!(crate::PonkoJoint3D, body_a));
    assert!(std::mem::offset_of!(CrabbyJoint3D, body_b) == std::mem::offset_of!(crate::PonkoJoint3D, body_b));
    assert!(std::mem::offset_of!(CrabbyJoint3D, frame_a) == std::mem::offset_of!(crate::PonkoJoint3D, frame_a));
    assert!(std::mem::offset_of!(CrabbyJoint3D, frame_b) == std::mem::offset_of!(crate::PonkoJoint3D, frame_b));
    assert!(std::mem::offset_of!(CrabbyJoint3D, axes) == std::mem::offset_of!(crate::PonkoJoint3D, axes));
    assert!(std::mem::offset_of!(CrabbyJoint3D, drives) == std::mem::offset_of!(crate::PonkoJoint3D, drives));
    assert!(
        std::mem::offset_of!(CrabbyJoint3D, contacts_enabled)
            == std::mem::offset_of!(crate::PonkoJoint3D, contacts_enabled)
    );
    assert!(std::mem::offset_of!(CrabbyJoint3D, swing_cone) == std::mem::offset_of!(crate::PonkoJoint3D, swing_cone));
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoCharacter3D as CrabbyCharacter3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoColliderFocus as CrabbyColliderFocus;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoCollisionMeshData as CrabbyCollisionMeshData;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoHit3D as CrabbyHit3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoQueryCachePolicy as CrabbyQueryCachePolicy;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoQueryCacheStats as CrabbyQueryCacheStats;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoQueryFilter3D as CrabbyQueryFilter3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoRagdollBone as CrabbyRagdollBone;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoTerrainCollider as CrabbyTerrainCollider;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyFieldType {
    CRABBY_FIELD_F32 = 0,

    CRABBY_FIELD_F64 = 1,

    CRABBY_FIELD_I8 = 2,

    CRABBY_FIELD_I16 = 3,

    CRABBY_FIELD_I32 = 4,

    CRABBY_FIELD_I64 = 5,

    CRABBY_FIELD_U8 = 6,

    CRABBY_FIELD_U16 = 7,

    CRABBY_FIELD_U32 = 8,

    CRABBY_FIELD_U64 = 9,

    CRABBY_FIELD_BOOL = 10,

    CRABBY_FIELD_VEC2 = 11,

    CRABBY_FIELD_COLOR = 12,

    CRABBY_FIELD_ENTITY = 13,
}
impl From<crate::PonkoFieldType> for CrabbyFieldType {
    fn from(value: crate::PonkoFieldType) -> Self {
        match value {
            crate::PonkoFieldType::PONKO_FIELD_F32 => Self::CRABBY_FIELD_F32,
            crate::PonkoFieldType::PONKO_FIELD_F64 => Self::CRABBY_FIELD_F64,
            crate::PonkoFieldType::PONKO_FIELD_I8 => Self::CRABBY_FIELD_I8,
            crate::PonkoFieldType::PONKO_FIELD_I16 => Self::CRABBY_FIELD_I16,
            crate::PonkoFieldType::PONKO_FIELD_I32 => Self::CRABBY_FIELD_I32,
            crate::PonkoFieldType::PONKO_FIELD_I64 => Self::CRABBY_FIELD_I64,
            crate::PonkoFieldType::PONKO_FIELD_U8 => Self::CRABBY_FIELD_U8,
            crate::PonkoFieldType::PONKO_FIELD_U16 => Self::CRABBY_FIELD_U16,
            crate::PonkoFieldType::PONKO_FIELD_U32 => Self::CRABBY_FIELD_U32,
            crate::PonkoFieldType::PONKO_FIELD_U64 => Self::CRABBY_FIELD_U64,
            crate::PonkoFieldType::PONKO_FIELD_BOOL => Self::CRABBY_FIELD_BOOL,
            crate::PonkoFieldType::PONKO_FIELD_VEC2 => Self::CRABBY_FIELD_VEC2,
            crate::PonkoFieldType::PONKO_FIELD_COLOR => Self::CRABBY_FIELD_COLOR,
            crate::PonkoFieldType::PONKO_FIELD_ENTITY => Self::CRABBY_FIELD_ENTITY,
        }
    }
}
impl From<CrabbyFieldType> for crate::PonkoFieldType {
    fn from(value: CrabbyFieldType) -> Self {
        match value {
            CrabbyFieldType::CRABBY_FIELD_F32 => Self::PONKO_FIELD_F32,
            CrabbyFieldType::CRABBY_FIELD_F64 => Self::PONKO_FIELD_F64,
            CrabbyFieldType::CRABBY_FIELD_I8 => Self::PONKO_FIELD_I8,
            CrabbyFieldType::CRABBY_FIELD_I16 => Self::PONKO_FIELD_I16,
            CrabbyFieldType::CRABBY_FIELD_I32 => Self::PONKO_FIELD_I32,
            CrabbyFieldType::CRABBY_FIELD_I64 => Self::PONKO_FIELD_I64,
            CrabbyFieldType::CRABBY_FIELD_U8 => Self::PONKO_FIELD_U8,
            CrabbyFieldType::CRABBY_FIELD_U16 => Self::PONKO_FIELD_U16,
            CrabbyFieldType::CRABBY_FIELD_U32 => Self::PONKO_FIELD_U32,
            CrabbyFieldType::CRABBY_FIELD_U64 => Self::PONKO_FIELD_U64,
            CrabbyFieldType::CRABBY_FIELD_BOOL => Self::PONKO_FIELD_BOOL,
            CrabbyFieldType::CRABBY_FIELD_VEC2 => Self::PONKO_FIELD_VEC2,
            CrabbyFieldType::CRABBY_FIELD_COLOR => Self::PONKO_FIELD_COLOR,
            CrabbyFieldType::CRABBY_FIELD_ENTITY => Self::PONKO_FIELD_ENTITY,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyFieldType>() == std::mem::size_of::<crate::PonkoFieldType>());
    assert!(std::mem::align_of::<CrabbyFieldType>() == std::mem::align_of::<crate::PonkoFieldType>());
    assert!(CrabbyFieldType::CRABBY_FIELD_F32 as u32 == crate::PonkoFieldType::PONKO_FIELD_F32 as u32);
    assert!(CrabbyFieldType::CRABBY_FIELD_F64 as u32 == crate::PonkoFieldType::PONKO_FIELD_F64 as u32);
    assert!(CrabbyFieldType::CRABBY_FIELD_I8 as u32 == crate::PonkoFieldType::PONKO_FIELD_I8 as u32);
    assert!(CrabbyFieldType::CRABBY_FIELD_I16 as u32 == crate::PonkoFieldType::PONKO_FIELD_I16 as u32);
    assert!(CrabbyFieldType::CRABBY_FIELD_I32 as u32 == crate::PonkoFieldType::PONKO_FIELD_I32 as u32);
    assert!(CrabbyFieldType::CRABBY_FIELD_I64 as u32 == crate::PonkoFieldType::PONKO_FIELD_I64 as u32);
    assert!(CrabbyFieldType::CRABBY_FIELD_U8 as u32 == crate::PonkoFieldType::PONKO_FIELD_U8 as u32);
    assert!(CrabbyFieldType::CRABBY_FIELD_U16 as u32 == crate::PonkoFieldType::PONKO_FIELD_U16 as u32);
    assert!(CrabbyFieldType::CRABBY_FIELD_U32 as u32 == crate::PonkoFieldType::PONKO_FIELD_U32 as u32);
    assert!(CrabbyFieldType::CRABBY_FIELD_U64 as u32 == crate::PonkoFieldType::PONKO_FIELD_U64 as u32);
    assert!(CrabbyFieldType::CRABBY_FIELD_BOOL as u32 == crate::PonkoFieldType::PONKO_FIELD_BOOL as u32);
    assert!(CrabbyFieldType::CRABBY_FIELD_VEC2 as u32 == crate::PonkoFieldType::PONKO_FIELD_VEC2 as u32);
    assert!(CrabbyFieldType::CRABBY_FIELD_COLOR as u32 == crate::PonkoFieldType::PONKO_FIELD_COLOR as u32);
    assert!(CrabbyFieldType::CRABBY_FIELD_ENTITY as u32 == crate::PonkoFieldType::PONKO_FIELD_ENTITY as u32);
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoAuthority as CrabbyAuthority;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoField as CrabbyField;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoUpdateStamp as CrabbyUpdateStamp;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyFieldType::CRABBY_FIELD_BOOL;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyFieldType::CRABBY_FIELD_COLOR;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyFieldType::CRABBY_FIELD_ENTITY;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyFieldType::CRABBY_FIELD_F32;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyFieldType::CRABBY_FIELD_F64;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyFieldType::CRABBY_FIELD_I8;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyFieldType::CRABBY_FIELD_I16;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyFieldType::CRABBY_FIELD_I32;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyFieldType::CRABBY_FIELD_I64;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyFieldType::CRABBY_FIELD_U8;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyFieldType::CRABBY_FIELD_U16;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyFieldType::CRABBY_FIELD_U32;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyFieldType::CRABBY_FIELD_U64;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyFieldType::CRABBY_FIELD_VEC2;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyAuthorityCheck {
    CRABBY_AUTHORITY_ACCEPT = 0,

    CRABBY_AUTHORITY_STALE = 1,

    CRABBY_AUTHORITY_WRONG_OWNER = 2,

    CRABBY_AUTHORITY_AHEAD = 3,
}
impl From<crate::PonkoAuthorityCheck> for CrabbyAuthorityCheck {
    fn from(value: crate::PonkoAuthorityCheck) -> Self {
        match value {
            crate::PonkoAuthorityCheck::PONKO_AUTHORITY_ACCEPT => Self::CRABBY_AUTHORITY_ACCEPT,
            crate::PonkoAuthorityCheck::PONKO_AUTHORITY_STALE => Self::CRABBY_AUTHORITY_STALE,
            crate::PonkoAuthorityCheck::PONKO_AUTHORITY_WRONG_OWNER => Self::CRABBY_AUTHORITY_WRONG_OWNER,
            crate::PonkoAuthorityCheck::PONKO_AUTHORITY_AHEAD => Self::CRABBY_AUTHORITY_AHEAD,
        }
    }
}
impl From<CrabbyAuthorityCheck> for crate::PonkoAuthorityCheck {
    fn from(value: CrabbyAuthorityCheck) -> Self {
        match value {
            CrabbyAuthorityCheck::CRABBY_AUTHORITY_ACCEPT => Self::PONKO_AUTHORITY_ACCEPT,
            CrabbyAuthorityCheck::CRABBY_AUTHORITY_STALE => Self::PONKO_AUTHORITY_STALE,
            CrabbyAuthorityCheck::CRABBY_AUTHORITY_WRONG_OWNER => Self::PONKO_AUTHORITY_WRONG_OWNER,
            CrabbyAuthorityCheck::CRABBY_AUTHORITY_AHEAD => Self::PONKO_AUTHORITY_AHEAD,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyAuthorityCheck>() == std::mem::size_of::<crate::PonkoAuthorityCheck>());
    assert!(std::mem::align_of::<CrabbyAuthorityCheck>() == std::mem::align_of::<crate::PonkoAuthorityCheck>());
    assert!(
        CrabbyAuthorityCheck::CRABBY_AUTHORITY_ACCEPT as u32
            == crate::PonkoAuthorityCheck::PONKO_AUTHORITY_ACCEPT as u32
    );
    assert!(
        CrabbyAuthorityCheck::CRABBY_AUTHORITY_STALE as u32 == crate::PonkoAuthorityCheck::PONKO_AUTHORITY_STALE as u32
    );
    assert!(
        CrabbyAuthorityCheck::CRABBY_AUTHORITY_WRONG_OWNER as u32
            == crate::PonkoAuthorityCheck::PONKO_AUTHORITY_WRONG_OWNER as u32
    );
    assert!(
        CrabbyAuthorityCheck::CRABBY_AUTHORITY_AHEAD as u32 == crate::PonkoAuthorityCheck::PONKO_AUTHORITY_AHEAD as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoLatestWins as CrabbyLatestWins;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoPose as CrabbyPose;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoSnapshotConfig as CrabbySnapshotConfig;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAuthorityCheck::CRABBY_AUTHORITY_ACCEPT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAuthorityCheck::CRABBY_AUTHORITY_AHEAD;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAuthorityCheck::CRABBY_AUTHORITY_STALE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAuthorityCheck::CRABBY_AUTHORITY_WRONG_OWNER;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbySnapshotPush {
    CRABBY_SNAPSHOT_ADDED = 0,

    CRABBY_SNAPSHOT_DUPLICATE = 1,

    CRABBY_SNAPSHOT_TOO_OLD = 2,

    CRABBY_SNAPSHOT_INVALID = 3,
}
impl From<crate::PonkoSnapshotPush> for CrabbySnapshotPush {
    fn from(value: crate::PonkoSnapshotPush) -> Self {
        match value {
            crate::PonkoSnapshotPush::PONKO_SNAPSHOT_ADDED => Self::CRABBY_SNAPSHOT_ADDED,
            crate::PonkoSnapshotPush::PONKO_SNAPSHOT_DUPLICATE => Self::CRABBY_SNAPSHOT_DUPLICATE,
            crate::PonkoSnapshotPush::PONKO_SNAPSHOT_TOO_OLD => Self::CRABBY_SNAPSHOT_TOO_OLD,
            crate::PonkoSnapshotPush::PONKO_SNAPSHOT_INVALID => Self::CRABBY_SNAPSHOT_INVALID,
        }
    }
}
impl From<CrabbySnapshotPush> for crate::PonkoSnapshotPush {
    fn from(value: CrabbySnapshotPush) -> Self {
        match value {
            CrabbySnapshotPush::CRABBY_SNAPSHOT_ADDED => Self::PONKO_SNAPSHOT_ADDED,
            CrabbySnapshotPush::CRABBY_SNAPSHOT_DUPLICATE => Self::PONKO_SNAPSHOT_DUPLICATE,
            CrabbySnapshotPush::CRABBY_SNAPSHOT_TOO_OLD => Self::PONKO_SNAPSHOT_TOO_OLD,
            CrabbySnapshotPush::CRABBY_SNAPSHOT_INVALID => Self::PONKO_SNAPSHOT_INVALID,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbySnapshotPush>() == std::mem::size_of::<crate::PonkoSnapshotPush>());
    assert!(std::mem::align_of::<CrabbySnapshotPush>() == std::mem::align_of::<crate::PonkoSnapshotPush>());
    assert!(CrabbySnapshotPush::CRABBY_SNAPSHOT_ADDED as u32 == crate::PonkoSnapshotPush::PONKO_SNAPSHOT_ADDED as u32);
    assert!(
        CrabbySnapshotPush::CRABBY_SNAPSHOT_DUPLICATE as u32
            == crate::PonkoSnapshotPush::PONKO_SNAPSHOT_DUPLICATE as u32
    );
    assert!(
        CrabbySnapshotPush::CRABBY_SNAPSHOT_TOO_OLD as u32 == crate::PonkoSnapshotPush::PONKO_SNAPSHOT_TOO_OLD as u32
    );
    assert!(
        CrabbySnapshotPush::CRABBY_SNAPSHOT_INVALID as u32 == crate::PonkoSnapshotPush::PONKO_SNAPSHOT_INVALID as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbySnapshotPush::CRABBY_SNAPSHOT_ADDED;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbySnapshotPush::CRABBY_SNAPSHOT_DUPLICATE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbySnapshotPush::CRABBY_SNAPSHOT_INVALID;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbySnapshotPush::CRABBY_SNAPSHOT_TOO_OLD;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbySnapshotState {
    CRABBY_SNAPSHOT_INTERPOLATED = 0,

    CRABBY_SNAPSHOT_EXTRAPOLATED = 1,

    CRABBY_SNAPSHOT_HELD = 2,
}
impl From<crate::PonkoSnapshotState> for CrabbySnapshotState {
    fn from(value: crate::PonkoSnapshotState) -> Self {
        match value {
            crate::PonkoSnapshotState::PONKO_SNAPSHOT_INTERPOLATED => Self::CRABBY_SNAPSHOT_INTERPOLATED,
            crate::PonkoSnapshotState::PONKO_SNAPSHOT_EXTRAPOLATED => Self::CRABBY_SNAPSHOT_EXTRAPOLATED,
            crate::PonkoSnapshotState::PONKO_SNAPSHOT_HELD => Self::CRABBY_SNAPSHOT_HELD,
        }
    }
}
impl From<CrabbySnapshotState> for crate::PonkoSnapshotState {
    fn from(value: CrabbySnapshotState) -> Self {
        match value {
            CrabbySnapshotState::CRABBY_SNAPSHOT_INTERPOLATED => Self::PONKO_SNAPSHOT_INTERPOLATED,
            CrabbySnapshotState::CRABBY_SNAPSHOT_EXTRAPOLATED => Self::PONKO_SNAPSHOT_EXTRAPOLATED,
            CrabbySnapshotState::CRABBY_SNAPSHOT_HELD => Self::PONKO_SNAPSHOT_HELD,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbySnapshotState>() == std::mem::size_of::<crate::PonkoSnapshotState>());
    assert!(std::mem::align_of::<CrabbySnapshotState>() == std::mem::align_of::<crate::PonkoSnapshotState>());
    assert!(
        CrabbySnapshotState::CRABBY_SNAPSHOT_INTERPOLATED as u32
            == crate::PonkoSnapshotState::PONKO_SNAPSHOT_INTERPOLATED as u32
    );
    assert!(
        CrabbySnapshotState::CRABBY_SNAPSHOT_EXTRAPOLATED as u32
            == crate::PonkoSnapshotState::PONKO_SNAPSHOT_EXTRAPOLATED as u32
    );
    assert!(CrabbySnapshotState::CRABBY_SNAPSHOT_HELD as u32 == crate::PonkoSnapshotState::PONKO_SNAPSHOT_HELD as u32);
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoInterestConfig as CrabbyInterestConfig;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoInterestEntity as CrabbyInterestEntity;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoInterestEntry as CrabbyInterestEntry;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoInterestTier as CrabbyInterestTier;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoInterestViewer as CrabbyInterestViewer;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoSnapshotStream as CrabbySnapshotStream;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbySnapshotState::CRABBY_SNAPSHOT_EXTRAPOLATED;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbySnapshotState::CRABBY_SNAPSHOT_HELD;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbySnapshotState::CRABBY_SNAPSHOT_INTERPOLATED;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyInterestChangeKind {
    CRABBY_INTEREST_ENTERED = 0,

    CRABBY_INTEREST_LEFT = 1,
}
impl From<crate::PonkoInterestChangeKind> for CrabbyInterestChangeKind {
    fn from(value: crate::PonkoInterestChangeKind) -> Self {
        match value {
            crate::PonkoInterestChangeKind::PONKO_INTEREST_ENTERED => Self::CRABBY_INTEREST_ENTERED,
            crate::PonkoInterestChangeKind::PONKO_INTEREST_LEFT => Self::CRABBY_INTEREST_LEFT,
        }
    }
}
impl From<CrabbyInterestChangeKind> for crate::PonkoInterestChangeKind {
    fn from(value: CrabbyInterestChangeKind) -> Self {
        match value {
            CrabbyInterestChangeKind::CRABBY_INTEREST_ENTERED => Self::PONKO_INTEREST_ENTERED,
            CrabbyInterestChangeKind::CRABBY_INTEREST_LEFT => Self::PONKO_INTEREST_LEFT,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyInterestChangeKind>() == std::mem::size_of::<crate::PonkoInterestChangeKind>());
    assert!(std::mem::align_of::<CrabbyInterestChangeKind>() == std::mem::align_of::<crate::PonkoInterestChangeKind>());
    assert!(
        CrabbyInterestChangeKind::CRABBY_INTEREST_ENTERED as u32
            == crate::PonkoInterestChangeKind::PONKO_INTEREST_ENTERED as u32
    );
    assert!(
        CrabbyInterestChangeKind::CRABBY_INTEREST_LEFT as u32
            == crate::PonkoInterestChangeKind::PONKO_INTEREST_LEFT as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyInterestChangeKind::CRABBY_INTEREST_ENTERED;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyInterestChangeKind::CRABBY_INTEREST_LEFT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrabbyInterestChange {
    pub peer: u64,

    pub entity: u64,

    pub kind: CrabbyInterestChangeKind,
}
impl From<crate::PonkoInterestChange> for CrabbyInterestChange {
    fn from(value: crate::PonkoInterestChange) -> Self {
        Self {
            peer: value.peer,
            entity: value.entity,
            kind: value.kind.into(),
        }
    }
}
impl From<CrabbyInterestChange> for crate::PonkoInterestChange {
    fn from(value: CrabbyInterestChange) -> Self {
        Self {
            peer: value.peer,
            entity: value.entity,
            kind: value.kind.into(),
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyInterestChange>() == std::mem::size_of::<crate::PonkoInterestChange>());
    assert!(std::mem::align_of::<CrabbyInterestChange>() == std::mem::align_of::<crate::PonkoInterestChange>());
    assert!(std::mem::offset_of!(CrabbyInterestChange, peer) == std::mem::offset_of!(crate::PonkoInterestChange, peer));
    assert!(
        std::mem::offset_of!(CrabbyInterestChange, entity) == std::mem::offset_of!(crate::PonkoInterestChange, entity)
    );
    assert!(std::mem::offset_of!(CrabbyInterestChange, kind) == std::mem::offset_of!(crate::PonkoInterestChange, kind));
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoInterest as CrabbyInterest;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyResidencyKind {
    CRABBY_RESIDENCY_MESH = 0,

    CRABBY_RESIDENCY_TEXTURE = 1,
}
impl From<crate::PonkoResidencyKind> for CrabbyResidencyKind {
    fn from(value: crate::PonkoResidencyKind) -> Self {
        match value {
            crate::PonkoResidencyKind::PONKO_RESIDENCY_MESH => Self::CRABBY_RESIDENCY_MESH,
            crate::PonkoResidencyKind::PONKO_RESIDENCY_TEXTURE => Self::CRABBY_RESIDENCY_TEXTURE,
        }
    }
}
impl From<CrabbyResidencyKind> for crate::PonkoResidencyKind {
    fn from(value: CrabbyResidencyKind) -> Self {
        match value {
            CrabbyResidencyKind::CRABBY_RESIDENCY_MESH => Self::PONKO_RESIDENCY_MESH,
            CrabbyResidencyKind::CRABBY_RESIDENCY_TEXTURE => Self::PONKO_RESIDENCY_TEXTURE,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyResidencyKind>() == std::mem::size_of::<crate::PonkoResidencyKind>());
    assert!(std::mem::align_of::<CrabbyResidencyKind>() == std::mem::align_of::<crate::PonkoResidencyKind>());
    assert!(
        CrabbyResidencyKind::CRABBY_RESIDENCY_MESH as u32 == crate::PonkoResidencyKind::PONKO_RESIDENCY_MESH as u32
    );
    assert!(
        CrabbyResidencyKind::CRABBY_RESIDENCY_TEXTURE as u32
            == crate::PonkoResidencyKind::PONKO_RESIDENCY_TEXTURE as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyResidencyKind::CRABBY_RESIDENCY_MESH;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyResidencyKind::CRABBY_RESIDENCY_TEXTURE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrabbyAssetState {
    CRABBY_ASSET_UNKNOWN = 0,

    CRABBY_ASSET_LOADING = 1,

    CRABBY_ASSET_UPLOADING = 2,

    CRABBY_ASSET_STREAMING = 3,

    CRABBY_ASSET_RESIDENT = 4,

    CRABBY_ASSET_EVICTED = 5,

    CRABBY_ASSET_FAILED = 6,
}
impl From<crate::PonkoAssetState> for CrabbyAssetState {
    fn from(value: crate::PonkoAssetState) -> Self {
        match value {
            crate::PonkoAssetState::PONKO_ASSET_UNKNOWN => Self::CRABBY_ASSET_UNKNOWN,
            crate::PonkoAssetState::PONKO_ASSET_LOADING => Self::CRABBY_ASSET_LOADING,
            crate::PonkoAssetState::PONKO_ASSET_UPLOADING => Self::CRABBY_ASSET_UPLOADING,
            crate::PonkoAssetState::PONKO_ASSET_STREAMING => Self::CRABBY_ASSET_STREAMING,
            crate::PonkoAssetState::PONKO_ASSET_RESIDENT => Self::CRABBY_ASSET_RESIDENT,
            crate::PonkoAssetState::PONKO_ASSET_EVICTED => Self::CRABBY_ASSET_EVICTED,
            crate::PonkoAssetState::PONKO_ASSET_FAILED => Self::CRABBY_ASSET_FAILED,
        }
    }
}
impl From<CrabbyAssetState> for crate::PonkoAssetState {
    fn from(value: CrabbyAssetState) -> Self {
        match value {
            CrabbyAssetState::CRABBY_ASSET_UNKNOWN => Self::PONKO_ASSET_UNKNOWN,
            CrabbyAssetState::CRABBY_ASSET_LOADING => Self::PONKO_ASSET_LOADING,
            CrabbyAssetState::CRABBY_ASSET_UPLOADING => Self::PONKO_ASSET_UPLOADING,
            CrabbyAssetState::CRABBY_ASSET_STREAMING => Self::PONKO_ASSET_STREAMING,
            CrabbyAssetState::CRABBY_ASSET_RESIDENT => Self::PONKO_ASSET_RESIDENT,
            CrabbyAssetState::CRABBY_ASSET_EVICTED => Self::PONKO_ASSET_EVICTED,
            CrabbyAssetState::CRABBY_ASSET_FAILED => Self::PONKO_ASSET_FAILED,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyAssetState>() == std::mem::size_of::<crate::PonkoAssetState>());
    assert!(std::mem::align_of::<CrabbyAssetState>() == std::mem::align_of::<crate::PonkoAssetState>());
    assert!(CrabbyAssetState::CRABBY_ASSET_UNKNOWN as u32 == crate::PonkoAssetState::PONKO_ASSET_UNKNOWN as u32);
    assert!(CrabbyAssetState::CRABBY_ASSET_LOADING as u32 == crate::PonkoAssetState::PONKO_ASSET_LOADING as u32);
    assert!(CrabbyAssetState::CRABBY_ASSET_UPLOADING as u32 == crate::PonkoAssetState::PONKO_ASSET_UPLOADING as u32);
    assert!(CrabbyAssetState::CRABBY_ASSET_STREAMING as u32 == crate::PonkoAssetState::PONKO_ASSET_STREAMING as u32);
    assert!(CrabbyAssetState::CRABBY_ASSET_RESIDENT as u32 == crate::PonkoAssetState::PONKO_ASSET_RESIDENT as u32);
    assert!(CrabbyAssetState::CRABBY_ASSET_EVICTED as u32 == crate::PonkoAssetState::PONKO_ASSET_EVICTED as u32);
    assert!(CrabbyAssetState::CRABBY_ASSET_FAILED as u32 == crate::PonkoAssetState::PONKO_ASSET_FAILED as u32);
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoCell as CrabbyCell;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoCellStreamer as CrabbyCellStreamer;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoClip as CrabbyClip;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoFarTerrain as CrabbyFarTerrain;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoJoint as CrabbyJoint;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoLine3D as CrabbyLine3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoResidencyStats as CrabbyResidencyStats;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoShaderStatus as CrabbyShaderStatus;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoSteamLobby as CrabbySteamLobby;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoTerrainTileData as CrabbyTerrainTileData;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoTrail3D as CrabbyTrail3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoUploadStats as CrabbyUploadStats;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAssetState::CRABBY_ASSET_EVICTED;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAssetState::CRABBY_ASSET_FAILED;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAssetState::CRABBY_ASSET_LOADING;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAssetState::CRABBY_ASSET_RESIDENT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAssetState::CRABBY_ASSET_STREAMING;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAssetState::CRABBY_ASSET_UNKNOWN;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyAssetState::CRABBY_ASSET_UPLOADING;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(u32)]
#[derive(Clone, Copy, Debug)]
#[allow(non_camel_case_types)]
pub enum CrabbyTextAlign3D {
    CRABBY_TEXT_ALIGN3D_LEFT = 0,

    CRABBY_TEXT_ALIGN3D_CENTER = 1,

    CRABBY_TEXT_ALIGN3D_RIGHT = 2,
}
impl From<crate::PonkoTextAlign3D> for CrabbyTextAlign3D {
    fn from(value: crate::PonkoTextAlign3D) -> Self {
        match value {
            crate::PonkoTextAlign3D::PONKO_TEXT_ALIGN3D_LEFT => Self::CRABBY_TEXT_ALIGN3D_LEFT,
            crate::PonkoTextAlign3D::PONKO_TEXT_ALIGN3D_CENTER => Self::CRABBY_TEXT_ALIGN3D_CENTER,
            crate::PonkoTextAlign3D::PONKO_TEXT_ALIGN3D_RIGHT => Self::CRABBY_TEXT_ALIGN3D_RIGHT,
        }
    }
}
impl From<CrabbyTextAlign3D> for crate::PonkoTextAlign3D {
    fn from(value: CrabbyTextAlign3D) -> Self {
        match value {
            CrabbyTextAlign3D::CRABBY_TEXT_ALIGN3D_LEFT => Self::PONKO_TEXT_ALIGN3D_LEFT,
            CrabbyTextAlign3D::CRABBY_TEXT_ALIGN3D_CENTER => Self::PONKO_TEXT_ALIGN3D_CENTER,
            CrabbyTextAlign3D::CRABBY_TEXT_ALIGN3D_RIGHT => Self::PONKO_TEXT_ALIGN3D_RIGHT,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyTextAlign3D>() == std::mem::size_of::<crate::PonkoTextAlign3D>());
    assert!(std::mem::align_of::<CrabbyTextAlign3D>() == std::mem::align_of::<crate::PonkoTextAlign3D>());
    assert!(
        CrabbyTextAlign3D::CRABBY_TEXT_ALIGN3D_LEFT as u32 == crate::PonkoTextAlign3D::PONKO_TEXT_ALIGN3D_LEFT as u32
    );
    assert!(
        CrabbyTextAlign3D::CRABBY_TEXT_ALIGN3D_CENTER as u32
            == crate::PonkoTextAlign3D::PONKO_TEXT_ALIGN3D_CENTER as u32
    );
    assert!(
        CrabbyTextAlign3D::CRABBY_TEXT_ALIGN3D_RIGHT as u32 == crate::PonkoTextAlign3D::PONKO_TEXT_ALIGN3D_RIGHT as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoAnimation as CrabbyAnimation;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoCamera3D as CrabbyCamera3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoColor as CrabbyColor;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoCompute as CrabbyCompute;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoComputePass as CrabbyComputePass;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoDecal3D as CrabbyDecal3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoDrawParams as CrabbyDrawParams;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoEffect as CrabbyEffect;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoEntity as CrabbyEntity;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoGpuBuffer as CrabbyGpuBuffer;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoGraphicsQuality as CrabbyGraphicsQuality;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoGridHit as CrabbyGridHit;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoGridItem as CrabbyGridItem;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoHeightfield as CrabbyHeightfield;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoInstanceBatch as CrabbyInstanceBatch;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoInstanceBatchOptions as CrabbyInstanceBatchOptions;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoInstanceLevel as CrabbyInstanceLevel;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoLighting as CrabbyLighting;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoLodLevel as CrabbyLodLevel;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoLodPick as CrabbyLodPick;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoLodRenderer as CrabbyLodRenderer;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoLodSettings as CrabbyLodSettings;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoMaterial as CrabbyMaterial;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoMaterialDesc as CrabbyMaterialDesc;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoMesh as CrabbyMesh;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoMeshData as CrabbyMeshData;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoMeshView as CrabbyMeshView;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoModel as CrabbyModel;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoPointLight3D as CrabbyPointLight3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoPost as CrabbyPost;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoQuat as CrabbyQuat;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoReceivesDecals as CrabbyReceivesDecals;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoRect as CrabbyRect;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoReflectionProbe3D as CrabbyReflectionProbe3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoScreenEffect as CrabbyScreenEffect;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoShader as CrabbyShader;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoSky as CrabbySky;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoSpotLight3D as CrabbySpotLight3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoSprite as CrabbySprite;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoSpriteSheet as CrabbySpriteSheet;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoTargetCamera as CrabbyTargetCamera;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoTerrainMaterialDesc as CrabbyTerrainMaterialDesc;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoTerrainTile as CrabbyTerrainTile;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoText3D as CrabbyText3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoTexture as CrabbyTexture;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoTextureArray as CrabbyTextureArray;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoTileMap as CrabbyTileMap;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoTransform2D as CrabbyTransform2D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoTransform3D as CrabbyTransform3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoVec2 as CrabbyVec2;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoVec3 as CrabbyVec3;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyTextAlign3D::CRABBY_TEXT_ALIGN3D_CENTER;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyTextAlign3D::CRABBY_TEXT_ALIGN3D_LEFT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyTextAlign3D::CRABBY_TEXT_ALIGN3D_RIGHT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiRole {
    #[default]
    CRABBY_UI_ROLE_GROUP,

    CRABBY_UI_ROLE_DIALOG,

    CRABBY_UI_ROLE_LABEL,

    CRABBY_UI_ROLE_BUTTON,

    CRABBY_UI_ROLE_IMAGE,

    CRABBY_UI_ROLE_SCROLL_VIEW,

    CRABBY_UI_ROLE_TEXT_INPUT,

    CRABBY_UI_ROLE_CHECKBOX,

    CRABBY_UI_ROLE_SLIDER,
}
impl From<crate::PonkoUiRole> for CrabbyUiRole {
    fn from(value: crate::PonkoUiRole) -> Self {
        match value {
            crate::PonkoUiRole::PONKO_UI_ROLE_GROUP => Self::CRABBY_UI_ROLE_GROUP,
            crate::PonkoUiRole::PONKO_UI_ROLE_DIALOG => Self::CRABBY_UI_ROLE_DIALOG,
            crate::PonkoUiRole::PONKO_UI_ROLE_LABEL => Self::CRABBY_UI_ROLE_LABEL,
            crate::PonkoUiRole::PONKO_UI_ROLE_BUTTON => Self::CRABBY_UI_ROLE_BUTTON,
            crate::PonkoUiRole::PONKO_UI_ROLE_IMAGE => Self::CRABBY_UI_ROLE_IMAGE,
            crate::PonkoUiRole::PONKO_UI_ROLE_SCROLL_VIEW => Self::CRABBY_UI_ROLE_SCROLL_VIEW,
            crate::PonkoUiRole::PONKO_UI_ROLE_TEXT_INPUT => Self::CRABBY_UI_ROLE_TEXT_INPUT,
            crate::PonkoUiRole::PONKO_UI_ROLE_CHECKBOX => Self::CRABBY_UI_ROLE_CHECKBOX,
            crate::PonkoUiRole::PONKO_UI_ROLE_SLIDER => Self::CRABBY_UI_ROLE_SLIDER,
        }
    }
}
impl From<CrabbyUiRole> for crate::PonkoUiRole {
    fn from(value: CrabbyUiRole) -> Self {
        match value {
            CrabbyUiRole::CRABBY_UI_ROLE_GROUP => Self::PONKO_UI_ROLE_GROUP,
            CrabbyUiRole::CRABBY_UI_ROLE_DIALOG => Self::PONKO_UI_ROLE_DIALOG,
            CrabbyUiRole::CRABBY_UI_ROLE_LABEL => Self::PONKO_UI_ROLE_LABEL,
            CrabbyUiRole::CRABBY_UI_ROLE_BUTTON => Self::PONKO_UI_ROLE_BUTTON,
            CrabbyUiRole::CRABBY_UI_ROLE_IMAGE => Self::PONKO_UI_ROLE_IMAGE,
            CrabbyUiRole::CRABBY_UI_ROLE_SCROLL_VIEW => Self::PONKO_UI_ROLE_SCROLL_VIEW,
            CrabbyUiRole::CRABBY_UI_ROLE_TEXT_INPUT => Self::PONKO_UI_ROLE_TEXT_INPUT,
            CrabbyUiRole::CRABBY_UI_ROLE_CHECKBOX => Self::PONKO_UI_ROLE_CHECKBOX,
            CrabbyUiRole::CRABBY_UI_ROLE_SLIDER => Self::PONKO_UI_ROLE_SLIDER,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyUiRole>() == std::mem::size_of::<crate::PonkoUiRole>());
    assert!(std::mem::align_of::<CrabbyUiRole>() == std::mem::align_of::<crate::PonkoUiRole>());
    assert!(CrabbyUiRole::CRABBY_UI_ROLE_GROUP as u32 == crate::PonkoUiRole::PONKO_UI_ROLE_GROUP as u32);
    assert!(CrabbyUiRole::CRABBY_UI_ROLE_DIALOG as u32 == crate::PonkoUiRole::PONKO_UI_ROLE_DIALOG as u32);
    assert!(CrabbyUiRole::CRABBY_UI_ROLE_LABEL as u32 == crate::PonkoUiRole::PONKO_UI_ROLE_LABEL as u32);
    assert!(CrabbyUiRole::CRABBY_UI_ROLE_BUTTON as u32 == crate::PonkoUiRole::PONKO_UI_ROLE_BUTTON as u32);
    assert!(CrabbyUiRole::CRABBY_UI_ROLE_IMAGE as u32 == crate::PonkoUiRole::PONKO_UI_ROLE_IMAGE as u32);
    assert!(CrabbyUiRole::CRABBY_UI_ROLE_SCROLL_VIEW as u32 == crate::PonkoUiRole::PONKO_UI_ROLE_SCROLL_VIEW as u32);
    assert!(CrabbyUiRole::CRABBY_UI_ROLE_TEXT_INPUT as u32 == crate::PonkoUiRole::PONKO_UI_ROLE_TEXT_INPUT as u32);
    assert!(CrabbyUiRole::CRABBY_UI_ROLE_CHECKBOX as u32 == crate::PonkoUiRole::PONKO_UI_ROLE_CHECKBOX as u32);
    assert!(CrabbyUiRole::CRABBY_UI_ROLE_SLIDER as u32 == crate::PonkoUiRole::PONKO_UI_ROLE_SLIDER as u32);
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiRole::CRABBY_UI_ROLE_BUTTON;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiRole::CRABBY_UI_ROLE_CHECKBOX;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiRole::CRABBY_UI_ROLE_DIALOG;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiRole::CRABBY_UI_ROLE_GROUP;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiRole::CRABBY_UI_ROLE_IMAGE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiRole::CRABBY_UI_ROLE_LABEL;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiRole::CRABBY_UI_ROLE_SCROLL_VIEW;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiRole::CRABBY_UI_ROLE_SLIDER;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiRole::CRABBY_UI_ROLE_TEXT_INPUT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiAccessValueKind {
    #[default]
    CRABBY_UI_ACCESS_VALUE_NONE,

    CRABBY_UI_ACCESS_VALUE_TEXT,

    CRABBY_UI_ACCESS_VALUE_CHECKED,

    CRABBY_UI_ACCESS_VALUE_RANGE,
}
impl From<crate::PonkoUiAccessValueKind> for CrabbyUiAccessValueKind {
    fn from(value: crate::PonkoUiAccessValueKind) -> Self {
        match value {
            crate::PonkoUiAccessValueKind::PONKO_UI_ACCESS_VALUE_NONE => Self::CRABBY_UI_ACCESS_VALUE_NONE,
            crate::PonkoUiAccessValueKind::PONKO_UI_ACCESS_VALUE_TEXT => Self::CRABBY_UI_ACCESS_VALUE_TEXT,
            crate::PonkoUiAccessValueKind::PONKO_UI_ACCESS_VALUE_CHECKED => Self::CRABBY_UI_ACCESS_VALUE_CHECKED,
            crate::PonkoUiAccessValueKind::PONKO_UI_ACCESS_VALUE_RANGE => Self::CRABBY_UI_ACCESS_VALUE_RANGE,
        }
    }
}
impl From<CrabbyUiAccessValueKind> for crate::PonkoUiAccessValueKind {
    fn from(value: CrabbyUiAccessValueKind) -> Self {
        match value {
            CrabbyUiAccessValueKind::CRABBY_UI_ACCESS_VALUE_NONE => Self::PONKO_UI_ACCESS_VALUE_NONE,
            CrabbyUiAccessValueKind::CRABBY_UI_ACCESS_VALUE_TEXT => Self::PONKO_UI_ACCESS_VALUE_TEXT,
            CrabbyUiAccessValueKind::CRABBY_UI_ACCESS_VALUE_CHECKED => Self::PONKO_UI_ACCESS_VALUE_CHECKED,
            CrabbyUiAccessValueKind::CRABBY_UI_ACCESS_VALUE_RANGE => Self::PONKO_UI_ACCESS_VALUE_RANGE,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyUiAccessValueKind>() == std::mem::size_of::<crate::PonkoUiAccessValueKind>());
    assert!(std::mem::align_of::<CrabbyUiAccessValueKind>() == std::mem::align_of::<crate::PonkoUiAccessValueKind>());
    assert!(
        CrabbyUiAccessValueKind::CRABBY_UI_ACCESS_VALUE_NONE as u32
            == crate::PonkoUiAccessValueKind::PONKO_UI_ACCESS_VALUE_NONE as u32
    );
    assert!(
        CrabbyUiAccessValueKind::CRABBY_UI_ACCESS_VALUE_TEXT as u32
            == crate::PonkoUiAccessValueKind::PONKO_UI_ACCESS_VALUE_TEXT as u32
    );
    assert!(
        CrabbyUiAccessValueKind::CRABBY_UI_ACCESS_VALUE_CHECKED as u32
            == crate::PonkoUiAccessValueKind::PONKO_UI_ACCESS_VALUE_CHECKED as u32
    );
    assert!(
        CrabbyUiAccessValueKind::CRABBY_UI_ACCESS_VALUE_RANGE as u32
            == crate::PonkoUiAccessValueKind::PONKO_UI_ACCESS_VALUE_RANGE as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAccessValueKind::CRABBY_UI_ACCESS_VALUE_CHECKED;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAccessValueKind::CRABBY_UI_ACCESS_VALUE_NONE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAccessValueKind::CRABBY_UI_ACCESS_VALUE_RANGE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAccessValueKind::CRABBY_UI_ACCESS_VALUE_TEXT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiLive {
    #[default]
    CRABBY_UI_LIVE_OFF,

    CRABBY_UI_LIVE_POLITE,

    CRABBY_UI_LIVE_ASSERTIVE,
}
impl From<crate::PonkoUiLive> for CrabbyUiLive {
    fn from(value: crate::PonkoUiLive) -> Self {
        match value {
            crate::PonkoUiLive::PONKO_UI_LIVE_OFF => Self::CRABBY_UI_LIVE_OFF,
            crate::PonkoUiLive::PONKO_UI_LIVE_POLITE => Self::CRABBY_UI_LIVE_POLITE,
            crate::PonkoUiLive::PONKO_UI_LIVE_ASSERTIVE => Self::CRABBY_UI_LIVE_ASSERTIVE,
        }
    }
}
impl From<CrabbyUiLive> for crate::PonkoUiLive {
    fn from(value: CrabbyUiLive) -> Self {
        match value {
            CrabbyUiLive::CRABBY_UI_LIVE_OFF => Self::PONKO_UI_LIVE_OFF,
            CrabbyUiLive::CRABBY_UI_LIVE_POLITE => Self::PONKO_UI_LIVE_POLITE,
            CrabbyUiLive::CRABBY_UI_LIVE_ASSERTIVE => Self::PONKO_UI_LIVE_ASSERTIVE,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyUiLive>() == std::mem::size_of::<crate::PonkoUiLive>());
    assert!(std::mem::align_of::<CrabbyUiLive>() == std::mem::align_of::<crate::PonkoUiLive>());
    assert!(CrabbyUiLive::CRABBY_UI_LIVE_OFF as u32 == crate::PonkoUiLive::PONKO_UI_LIVE_OFF as u32);
    assert!(CrabbyUiLive::CRABBY_UI_LIVE_POLITE as u32 == crate::PonkoUiLive::PONKO_UI_LIVE_POLITE as u32);
    assert!(CrabbyUiLive::CRABBY_UI_LIVE_ASSERTIVE as u32 == crate::PonkoUiLive::PONKO_UI_LIVE_ASSERTIVE as u32);
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiLive::CRABBY_UI_LIVE_ASSERTIVE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiLive::CRABBY_UI_LIVE_OFF;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiLive::CRABBY_UI_LIVE_POLITE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiAccessActionKind {
    #[default]
    CRABBY_UI_ACCESS_ACTION_FOCUS,

    CRABBY_UI_ACCESS_ACTION_ACTIVATE,

    CRABBY_UI_ACCESS_ACTION_SET_VALUE,

    CRABBY_UI_ACCESS_ACTION_INCREMENT,

    CRABBY_UI_ACCESS_ACTION_DECREMENT,

    CRABBY_UI_ACCESS_ACTION_SET_TEXT,
}
impl From<crate::PonkoUiAccessActionKind> for CrabbyUiAccessActionKind {
    fn from(value: crate::PonkoUiAccessActionKind) -> Self {
        match value {
            crate::PonkoUiAccessActionKind::PONKO_UI_ACCESS_ACTION_FOCUS => Self::CRABBY_UI_ACCESS_ACTION_FOCUS,
            crate::PonkoUiAccessActionKind::PONKO_UI_ACCESS_ACTION_ACTIVATE => Self::CRABBY_UI_ACCESS_ACTION_ACTIVATE,
            crate::PonkoUiAccessActionKind::PONKO_UI_ACCESS_ACTION_SET_VALUE => Self::CRABBY_UI_ACCESS_ACTION_SET_VALUE,
            crate::PonkoUiAccessActionKind::PONKO_UI_ACCESS_ACTION_INCREMENT => Self::CRABBY_UI_ACCESS_ACTION_INCREMENT,
            crate::PonkoUiAccessActionKind::PONKO_UI_ACCESS_ACTION_DECREMENT => Self::CRABBY_UI_ACCESS_ACTION_DECREMENT,
            crate::PonkoUiAccessActionKind::PONKO_UI_ACCESS_ACTION_SET_TEXT => Self::CRABBY_UI_ACCESS_ACTION_SET_TEXT,
        }
    }
}
impl From<CrabbyUiAccessActionKind> for crate::PonkoUiAccessActionKind {
    fn from(value: CrabbyUiAccessActionKind) -> Self {
        match value {
            CrabbyUiAccessActionKind::CRABBY_UI_ACCESS_ACTION_FOCUS => Self::PONKO_UI_ACCESS_ACTION_FOCUS,
            CrabbyUiAccessActionKind::CRABBY_UI_ACCESS_ACTION_ACTIVATE => Self::PONKO_UI_ACCESS_ACTION_ACTIVATE,
            CrabbyUiAccessActionKind::CRABBY_UI_ACCESS_ACTION_SET_VALUE => Self::PONKO_UI_ACCESS_ACTION_SET_VALUE,
            CrabbyUiAccessActionKind::CRABBY_UI_ACCESS_ACTION_INCREMENT => Self::PONKO_UI_ACCESS_ACTION_INCREMENT,
            CrabbyUiAccessActionKind::CRABBY_UI_ACCESS_ACTION_DECREMENT => Self::PONKO_UI_ACCESS_ACTION_DECREMENT,
            CrabbyUiAccessActionKind::CRABBY_UI_ACCESS_ACTION_SET_TEXT => Self::PONKO_UI_ACCESS_ACTION_SET_TEXT,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyUiAccessActionKind>() == std::mem::size_of::<crate::PonkoUiAccessActionKind>());
    assert!(std::mem::align_of::<CrabbyUiAccessActionKind>() == std::mem::align_of::<crate::PonkoUiAccessActionKind>());
    assert!(
        CrabbyUiAccessActionKind::CRABBY_UI_ACCESS_ACTION_FOCUS as u32
            == crate::PonkoUiAccessActionKind::PONKO_UI_ACCESS_ACTION_FOCUS as u32
    );
    assert!(
        CrabbyUiAccessActionKind::CRABBY_UI_ACCESS_ACTION_ACTIVATE as u32
            == crate::PonkoUiAccessActionKind::PONKO_UI_ACCESS_ACTION_ACTIVATE as u32
    );
    assert!(
        CrabbyUiAccessActionKind::CRABBY_UI_ACCESS_ACTION_SET_VALUE as u32
            == crate::PonkoUiAccessActionKind::PONKO_UI_ACCESS_ACTION_SET_VALUE as u32
    );
    assert!(
        CrabbyUiAccessActionKind::CRABBY_UI_ACCESS_ACTION_INCREMENT as u32
            == crate::PonkoUiAccessActionKind::PONKO_UI_ACCESS_ACTION_INCREMENT as u32
    );
    assert!(
        CrabbyUiAccessActionKind::CRABBY_UI_ACCESS_ACTION_DECREMENT as u32
            == crate::PonkoUiAccessActionKind::PONKO_UI_ACCESS_ACTION_DECREMENT as u32
    );
    assert!(
        CrabbyUiAccessActionKind::CRABBY_UI_ACCESS_ACTION_SET_TEXT as u32
            == crate::PonkoUiAccessActionKind::PONKO_UI_ACCESS_ACTION_SET_TEXT as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAccessActionKind::CRABBY_UI_ACCESS_ACTION_ACTIVATE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAccessActionKind::CRABBY_UI_ACCESS_ACTION_DECREMENT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAccessActionKind::CRABBY_UI_ACCESS_ACTION_FOCUS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAccessActionKind::CRABBY_UI_ACCESS_ACTION_INCREMENT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAccessActionKind::CRABBY_UI_ACCESS_ACTION_SET_TEXT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAccessActionKind::CRABBY_UI_ACCESS_ACTION_SET_VALUE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CrabbyUiAccessNode {
    pub id: u64,

    pub parent: u64,

    pub selection_anchor: u64,

    pub selection_caret: u64,

    pub rect: CrabbyRect,

    pub role: CrabbyUiRole,

    pub value_kind: CrabbyUiAccessValueKind,

    pub value: f32,

    pub live: CrabbyUiLive,

    pub depth: u32,

    pub child_count: u32,

    pub has_selection: bool,

    pub focusable: bool,

    pub focused: bool,

    pub disabled: bool,

    pub inert: bool,

    pub modal: bool,
}
impl From<crate::PonkoUiAccessNode> for CrabbyUiAccessNode {
    fn from(value: crate::PonkoUiAccessNode) -> Self {
        Self {
            id: value.id,
            parent: value.parent,
            selection_anchor: value.selection_anchor,
            selection_caret: value.selection_caret,
            rect: value.rect,
            role: value.role.into(),
            value_kind: value.value_kind.into(),
            value: value.value,
            live: value.live.into(),
            depth: value.depth,
            child_count: value.child_count,
            has_selection: value.has_selection,
            focusable: value.focusable,
            focused: value.focused,
            disabled: value.disabled,
            inert: value.inert,
            modal: value.modal,
        }
    }
}
impl From<CrabbyUiAccessNode> for crate::PonkoUiAccessNode {
    fn from(value: CrabbyUiAccessNode) -> Self {
        Self {
            id: value.id,
            parent: value.parent,
            selection_anchor: value.selection_anchor,
            selection_caret: value.selection_caret,
            rect: value.rect,
            role: value.role.into(),
            value_kind: value.value_kind.into(),
            value: value.value,
            live: value.live.into(),
            depth: value.depth,
            child_count: value.child_count,
            has_selection: value.has_selection,
            focusable: value.focusable,
            focused: value.focused,
            disabled: value.disabled,
            inert: value.inert,
            modal: value.modal,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyUiAccessNode>() == std::mem::size_of::<crate::PonkoUiAccessNode>());
    assert!(std::mem::align_of::<CrabbyUiAccessNode>() == std::mem::align_of::<crate::PonkoUiAccessNode>());
    assert!(std::mem::offset_of!(CrabbyUiAccessNode, id) == std::mem::offset_of!(crate::PonkoUiAccessNode, id));
    assert!(std::mem::offset_of!(CrabbyUiAccessNode, parent) == std::mem::offset_of!(crate::PonkoUiAccessNode, parent));
    assert!(
        std::mem::offset_of!(CrabbyUiAccessNode, selection_anchor)
            == std::mem::offset_of!(crate::PonkoUiAccessNode, selection_anchor)
    );
    assert!(
        std::mem::offset_of!(CrabbyUiAccessNode, selection_caret)
            == std::mem::offset_of!(crate::PonkoUiAccessNode, selection_caret)
    );
    assert!(std::mem::offset_of!(CrabbyUiAccessNode, rect) == std::mem::offset_of!(crate::PonkoUiAccessNode, rect));
    assert!(std::mem::offset_of!(CrabbyUiAccessNode, role) == std::mem::offset_of!(crate::PonkoUiAccessNode, role));
    assert!(
        std::mem::offset_of!(CrabbyUiAccessNode, value_kind)
            == std::mem::offset_of!(crate::PonkoUiAccessNode, value_kind)
    );
    assert!(std::mem::offset_of!(CrabbyUiAccessNode, value) == std::mem::offset_of!(crate::PonkoUiAccessNode, value));
    assert!(std::mem::offset_of!(CrabbyUiAccessNode, live) == std::mem::offset_of!(crate::PonkoUiAccessNode, live));
    assert!(std::mem::offset_of!(CrabbyUiAccessNode, depth) == std::mem::offset_of!(crate::PonkoUiAccessNode, depth));
    assert!(
        std::mem::offset_of!(CrabbyUiAccessNode, child_count)
            == std::mem::offset_of!(crate::PonkoUiAccessNode, child_count)
    );
    assert!(
        std::mem::offset_of!(CrabbyUiAccessNode, has_selection)
            == std::mem::offset_of!(crate::PonkoUiAccessNode, has_selection)
    );
    assert!(
        std::mem::offset_of!(CrabbyUiAccessNode, focusable)
            == std::mem::offset_of!(crate::PonkoUiAccessNode, focusable)
    );
    assert!(
        std::mem::offset_of!(CrabbyUiAccessNode, focused) == std::mem::offset_of!(crate::PonkoUiAccessNode, focused)
    );
    assert!(
        std::mem::offset_of!(CrabbyUiAccessNode, disabled) == std::mem::offset_of!(crate::PonkoUiAccessNode, disabled)
    );
    assert!(std::mem::offset_of!(CrabbyUiAccessNode, inert) == std::mem::offset_of!(crate::PonkoUiAccessNode, inert));
    assert!(std::mem::offset_of!(CrabbyUiAccessNode, modal) == std::mem::offset_of!(crate::PonkoUiAccessNode, modal));
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoUiAccess as CrabbyUiAccess;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoUiAccessChanges as CrabbyUiAccessChanges;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiDisplay {
    CRABBY_UI_DISPLAY_FLEX,

    CRABBY_UI_DISPLAY_GRID,

    CRABBY_UI_DISPLAY_NONE,
}
impl From<crate::PonkoUiDisplay> for CrabbyUiDisplay {
    fn from(value: crate::PonkoUiDisplay) -> Self {
        match value {
            crate::PonkoUiDisplay::PONKO_UI_DISPLAY_FLEX => Self::CRABBY_UI_DISPLAY_FLEX,
            crate::PonkoUiDisplay::PONKO_UI_DISPLAY_GRID => Self::CRABBY_UI_DISPLAY_GRID,
            crate::PonkoUiDisplay::PONKO_UI_DISPLAY_NONE => Self::CRABBY_UI_DISPLAY_NONE,
        }
    }
}
impl From<CrabbyUiDisplay> for crate::PonkoUiDisplay {
    fn from(value: CrabbyUiDisplay) -> Self {
        match value {
            CrabbyUiDisplay::CRABBY_UI_DISPLAY_FLEX => Self::PONKO_UI_DISPLAY_FLEX,
            CrabbyUiDisplay::CRABBY_UI_DISPLAY_GRID => Self::PONKO_UI_DISPLAY_GRID,
            CrabbyUiDisplay::CRABBY_UI_DISPLAY_NONE => Self::PONKO_UI_DISPLAY_NONE,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyUiDisplay>() == std::mem::size_of::<crate::PonkoUiDisplay>());
    assert!(std::mem::align_of::<CrabbyUiDisplay>() == std::mem::align_of::<crate::PonkoUiDisplay>());
    assert!(CrabbyUiDisplay::CRABBY_UI_DISPLAY_FLEX as u32 == crate::PonkoUiDisplay::PONKO_UI_DISPLAY_FLEX as u32);
    assert!(CrabbyUiDisplay::CRABBY_UI_DISPLAY_GRID as u32 == crate::PonkoUiDisplay::PONKO_UI_DISPLAY_GRID as u32);
    assert!(CrabbyUiDisplay::CRABBY_UI_DISPLAY_NONE as u32 == crate::PonkoUiDisplay::PONKO_UI_DISPLAY_NONE as u32);
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiDisplay::CRABBY_UI_DISPLAY_FLEX;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiDisplay::CRABBY_UI_DISPLAY_GRID;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiDisplay::CRABBY_UI_DISPLAY_NONE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiPosition {
    CRABBY_UI_POSITION_STATIC,

    CRABBY_UI_POSITION_RELATIVE,

    CRABBY_UI_POSITION_ABSOLUTE,

    CRABBY_UI_POSITION_FIXED,
}
impl From<crate::PonkoUiPosition> for CrabbyUiPosition {
    fn from(value: crate::PonkoUiPosition) -> Self {
        match value {
            crate::PonkoUiPosition::PONKO_UI_POSITION_STATIC => Self::CRABBY_UI_POSITION_STATIC,
            crate::PonkoUiPosition::PONKO_UI_POSITION_RELATIVE => Self::CRABBY_UI_POSITION_RELATIVE,
            crate::PonkoUiPosition::PONKO_UI_POSITION_ABSOLUTE => Self::CRABBY_UI_POSITION_ABSOLUTE,
            crate::PonkoUiPosition::PONKO_UI_POSITION_FIXED => Self::CRABBY_UI_POSITION_FIXED,
        }
    }
}
impl From<CrabbyUiPosition> for crate::PonkoUiPosition {
    fn from(value: CrabbyUiPosition) -> Self {
        match value {
            CrabbyUiPosition::CRABBY_UI_POSITION_STATIC => Self::PONKO_UI_POSITION_STATIC,
            CrabbyUiPosition::CRABBY_UI_POSITION_RELATIVE => Self::PONKO_UI_POSITION_RELATIVE,
            CrabbyUiPosition::CRABBY_UI_POSITION_ABSOLUTE => Self::PONKO_UI_POSITION_ABSOLUTE,
            CrabbyUiPosition::CRABBY_UI_POSITION_FIXED => Self::PONKO_UI_POSITION_FIXED,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyUiPosition>() == std::mem::size_of::<crate::PonkoUiPosition>());
    assert!(std::mem::align_of::<CrabbyUiPosition>() == std::mem::align_of::<crate::PonkoUiPosition>());
    assert!(
        CrabbyUiPosition::CRABBY_UI_POSITION_STATIC as u32 == crate::PonkoUiPosition::PONKO_UI_POSITION_STATIC as u32
    );
    assert!(
        CrabbyUiPosition::CRABBY_UI_POSITION_RELATIVE as u32
            == crate::PonkoUiPosition::PONKO_UI_POSITION_RELATIVE as u32
    );
    assert!(
        CrabbyUiPosition::CRABBY_UI_POSITION_ABSOLUTE as u32
            == crate::PonkoUiPosition::PONKO_UI_POSITION_ABSOLUTE as u32
    );
    assert!(
        CrabbyUiPosition::CRABBY_UI_POSITION_FIXED as u32 == crate::PonkoUiPosition::PONKO_UI_POSITION_FIXED as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiPosition::CRABBY_UI_POSITION_ABSOLUTE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiPosition::CRABBY_UI_POSITION_FIXED;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiPosition::CRABBY_UI_POSITION_RELATIVE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiPosition::CRABBY_UI_POSITION_STATIC;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiFlexDirection {
    CRABBY_UI_FLEX_ROW,

    CRABBY_UI_FLEX_COLUMN,

    CRABBY_UI_FLEX_ROW_REVERSE,

    CRABBY_UI_FLEX_COLUMN_REVERSE,
}
impl From<crate::PonkoUiFlexDirection> for CrabbyUiFlexDirection {
    fn from(value: crate::PonkoUiFlexDirection) -> Self {
        match value {
            crate::PonkoUiFlexDirection::PONKO_UI_FLEX_ROW => Self::CRABBY_UI_FLEX_ROW,
            crate::PonkoUiFlexDirection::PONKO_UI_FLEX_COLUMN => Self::CRABBY_UI_FLEX_COLUMN,
            crate::PonkoUiFlexDirection::PONKO_UI_FLEX_ROW_REVERSE => Self::CRABBY_UI_FLEX_ROW_REVERSE,
            crate::PonkoUiFlexDirection::PONKO_UI_FLEX_COLUMN_REVERSE => Self::CRABBY_UI_FLEX_COLUMN_REVERSE,
        }
    }
}
impl From<CrabbyUiFlexDirection> for crate::PonkoUiFlexDirection {
    fn from(value: CrabbyUiFlexDirection) -> Self {
        match value {
            CrabbyUiFlexDirection::CRABBY_UI_FLEX_ROW => Self::PONKO_UI_FLEX_ROW,
            CrabbyUiFlexDirection::CRABBY_UI_FLEX_COLUMN => Self::PONKO_UI_FLEX_COLUMN,
            CrabbyUiFlexDirection::CRABBY_UI_FLEX_ROW_REVERSE => Self::PONKO_UI_FLEX_ROW_REVERSE,
            CrabbyUiFlexDirection::CRABBY_UI_FLEX_COLUMN_REVERSE => Self::PONKO_UI_FLEX_COLUMN_REVERSE,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyUiFlexDirection>() == std::mem::size_of::<crate::PonkoUiFlexDirection>());
    assert!(std::mem::align_of::<CrabbyUiFlexDirection>() == std::mem::align_of::<crate::PonkoUiFlexDirection>());
    assert!(CrabbyUiFlexDirection::CRABBY_UI_FLEX_ROW as u32 == crate::PonkoUiFlexDirection::PONKO_UI_FLEX_ROW as u32);
    assert!(
        CrabbyUiFlexDirection::CRABBY_UI_FLEX_COLUMN as u32 == crate::PonkoUiFlexDirection::PONKO_UI_FLEX_COLUMN as u32
    );
    assert!(
        CrabbyUiFlexDirection::CRABBY_UI_FLEX_ROW_REVERSE as u32
            == crate::PonkoUiFlexDirection::PONKO_UI_FLEX_ROW_REVERSE as u32
    );
    assert!(
        CrabbyUiFlexDirection::CRABBY_UI_FLEX_COLUMN_REVERSE as u32
            == crate::PonkoUiFlexDirection::PONKO_UI_FLEX_COLUMN_REVERSE as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiFlexDirection::CRABBY_UI_FLEX_COLUMN;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiFlexDirection::CRABBY_UI_FLEX_COLUMN_REVERSE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiFlexDirection::CRABBY_UI_FLEX_ROW;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiFlexDirection::CRABBY_UI_FLEX_ROW_REVERSE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiFlexWrap {
    CRABBY_UI_FLEX_NO_WRAP,

    CRABBY_UI_FLEX_WRAP,

    CRABBY_UI_FLEX_WRAP_REVERSE,
}
impl From<crate::PonkoUiFlexWrap> for CrabbyUiFlexWrap {
    fn from(value: crate::PonkoUiFlexWrap) -> Self {
        match value {
            crate::PonkoUiFlexWrap::PONKO_UI_FLEX_NO_WRAP => Self::CRABBY_UI_FLEX_NO_WRAP,
            crate::PonkoUiFlexWrap::PONKO_UI_FLEX_WRAP => Self::CRABBY_UI_FLEX_WRAP,
            crate::PonkoUiFlexWrap::PONKO_UI_FLEX_WRAP_REVERSE => Self::CRABBY_UI_FLEX_WRAP_REVERSE,
        }
    }
}
impl From<CrabbyUiFlexWrap> for crate::PonkoUiFlexWrap {
    fn from(value: CrabbyUiFlexWrap) -> Self {
        match value {
            CrabbyUiFlexWrap::CRABBY_UI_FLEX_NO_WRAP => Self::PONKO_UI_FLEX_NO_WRAP,
            CrabbyUiFlexWrap::CRABBY_UI_FLEX_WRAP => Self::PONKO_UI_FLEX_WRAP,
            CrabbyUiFlexWrap::CRABBY_UI_FLEX_WRAP_REVERSE => Self::PONKO_UI_FLEX_WRAP_REVERSE,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyUiFlexWrap>() == std::mem::size_of::<crate::PonkoUiFlexWrap>());
    assert!(std::mem::align_of::<CrabbyUiFlexWrap>() == std::mem::align_of::<crate::PonkoUiFlexWrap>());
    assert!(CrabbyUiFlexWrap::CRABBY_UI_FLEX_NO_WRAP as u32 == crate::PonkoUiFlexWrap::PONKO_UI_FLEX_NO_WRAP as u32);
    assert!(CrabbyUiFlexWrap::CRABBY_UI_FLEX_WRAP as u32 == crate::PonkoUiFlexWrap::PONKO_UI_FLEX_WRAP as u32);
    assert!(
        CrabbyUiFlexWrap::CRABBY_UI_FLEX_WRAP_REVERSE as u32
            == crate::PonkoUiFlexWrap::PONKO_UI_FLEX_WRAP_REVERSE as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiFlexWrap::CRABBY_UI_FLEX_NO_WRAP;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiFlexWrap::CRABBY_UI_FLEX_WRAP;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiFlexWrap::CRABBY_UI_FLEX_WRAP_REVERSE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiAlign {
    CRABBY_UI_ALIGN_NORMAL,

    CRABBY_UI_ALIGN_START,

    CRABBY_UI_ALIGN_END,

    CRABBY_UI_ALIGN_FLEX_START,

    CRABBY_UI_ALIGN_FLEX_END,

    CRABBY_UI_ALIGN_CENTER,

    CRABBY_UI_ALIGN_BASELINE,

    CRABBY_UI_ALIGN_STRETCH,
}
impl From<crate::PonkoUiAlign> for CrabbyUiAlign {
    fn from(value: crate::PonkoUiAlign) -> Self {
        match value {
            crate::PonkoUiAlign::PONKO_UI_ALIGN_NORMAL => Self::CRABBY_UI_ALIGN_NORMAL,
            crate::PonkoUiAlign::PONKO_UI_ALIGN_START => Self::CRABBY_UI_ALIGN_START,
            crate::PonkoUiAlign::PONKO_UI_ALIGN_END => Self::CRABBY_UI_ALIGN_END,
            crate::PonkoUiAlign::PONKO_UI_ALIGN_FLEX_START => Self::CRABBY_UI_ALIGN_FLEX_START,
            crate::PonkoUiAlign::PONKO_UI_ALIGN_FLEX_END => Self::CRABBY_UI_ALIGN_FLEX_END,
            crate::PonkoUiAlign::PONKO_UI_ALIGN_CENTER => Self::CRABBY_UI_ALIGN_CENTER,
            crate::PonkoUiAlign::PONKO_UI_ALIGN_BASELINE => Self::CRABBY_UI_ALIGN_BASELINE,
            crate::PonkoUiAlign::PONKO_UI_ALIGN_STRETCH => Self::CRABBY_UI_ALIGN_STRETCH,
        }
    }
}
impl From<CrabbyUiAlign> for crate::PonkoUiAlign {
    fn from(value: CrabbyUiAlign) -> Self {
        match value {
            CrabbyUiAlign::CRABBY_UI_ALIGN_NORMAL => Self::PONKO_UI_ALIGN_NORMAL,
            CrabbyUiAlign::CRABBY_UI_ALIGN_START => Self::PONKO_UI_ALIGN_START,
            CrabbyUiAlign::CRABBY_UI_ALIGN_END => Self::PONKO_UI_ALIGN_END,
            CrabbyUiAlign::CRABBY_UI_ALIGN_FLEX_START => Self::PONKO_UI_ALIGN_FLEX_START,
            CrabbyUiAlign::CRABBY_UI_ALIGN_FLEX_END => Self::PONKO_UI_ALIGN_FLEX_END,
            CrabbyUiAlign::CRABBY_UI_ALIGN_CENTER => Self::PONKO_UI_ALIGN_CENTER,
            CrabbyUiAlign::CRABBY_UI_ALIGN_BASELINE => Self::PONKO_UI_ALIGN_BASELINE,
            CrabbyUiAlign::CRABBY_UI_ALIGN_STRETCH => Self::PONKO_UI_ALIGN_STRETCH,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyUiAlign>() == std::mem::size_of::<crate::PonkoUiAlign>());
    assert!(std::mem::align_of::<CrabbyUiAlign>() == std::mem::align_of::<crate::PonkoUiAlign>());
    assert!(CrabbyUiAlign::CRABBY_UI_ALIGN_NORMAL as u32 == crate::PonkoUiAlign::PONKO_UI_ALIGN_NORMAL as u32);
    assert!(CrabbyUiAlign::CRABBY_UI_ALIGN_START as u32 == crate::PonkoUiAlign::PONKO_UI_ALIGN_START as u32);
    assert!(CrabbyUiAlign::CRABBY_UI_ALIGN_END as u32 == crate::PonkoUiAlign::PONKO_UI_ALIGN_END as u32);
    assert!(CrabbyUiAlign::CRABBY_UI_ALIGN_FLEX_START as u32 == crate::PonkoUiAlign::PONKO_UI_ALIGN_FLEX_START as u32);
    assert!(CrabbyUiAlign::CRABBY_UI_ALIGN_FLEX_END as u32 == crate::PonkoUiAlign::PONKO_UI_ALIGN_FLEX_END as u32);
    assert!(CrabbyUiAlign::CRABBY_UI_ALIGN_CENTER as u32 == crate::PonkoUiAlign::PONKO_UI_ALIGN_CENTER as u32);
    assert!(CrabbyUiAlign::CRABBY_UI_ALIGN_BASELINE as u32 == crate::PonkoUiAlign::PONKO_UI_ALIGN_BASELINE as u32);
    assert!(CrabbyUiAlign::CRABBY_UI_ALIGN_STRETCH as u32 == crate::PonkoUiAlign::PONKO_UI_ALIGN_STRETCH as u32);
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAlign::CRABBY_UI_ALIGN_BASELINE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAlign::CRABBY_UI_ALIGN_CENTER;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAlign::CRABBY_UI_ALIGN_END;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAlign::CRABBY_UI_ALIGN_FLEX_END;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAlign::CRABBY_UI_ALIGN_FLEX_START;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAlign::CRABBY_UI_ALIGN_NORMAL;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAlign::CRABBY_UI_ALIGN_START;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAlign::CRABBY_UI_ALIGN_STRETCH;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiAlignContent {
    CRABBY_UI_ALIGN_CONTENT_NORMAL,

    CRABBY_UI_ALIGN_CONTENT_START,

    CRABBY_UI_ALIGN_CONTENT_END,

    CRABBY_UI_ALIGN_CONTENT_FLEX_START,

    CRABBY_UI_ALIGN_CONTENT_FLEX_END,

    CRABBY_UI_ALIGN_CONTENT_CENTER,

    CRABBY_UI_ALIGN_CONTENT_STRETCH,

    CRABBY_UI_ALIGN_CONTENT_SPACE_BETWEEN,

    CRABBY_UI_ALIGN_CONTENT_SPACE_EVENLY,

    CRABBY_UI_ALIGN_CONTENT_SPACE_AROUND,
}
impl From<crate::PonkoUiAlignContent> for CrabbyUiAlignContent {
    fn from(value: crate::PonkoUiAlignContent) -> Self {
        match value {
            crate::PonkoUiAlignContent::PONKO_UI_ALIGN_CONTENT_NORMAL => Self::CRABBY_UI_ALIGN_CONTENT_NORMAL,
            crate::PonkoUiAlignContent::PONKO_UI_ALIGN_CONTENT_START => Self::CRABBY_UI_ALIGN_CONTENT_START,
            crate::PonkoUiAlignContent::PONKO_UI_ALIGN_CONTENT_END => Self::CRABBY_UI_ALIGN_CONTENT_END,
            crate::PonkoUiAlignContent::PONKO_UI_ALIGN_CONTENT_FLEX_START => Self::CRABBY_UI_ALIGN_CONTENT_FLEX_START,
            crate::PonkoUiAlignContent::PONKO_UI_ALIGN_CONTENT_FLEX_END => Self::CRABBY_UI_ALIGN_CONTENT_FLEX_END,
            crate::PonkoUiAlignContent::PONKO_UI_ALIGN_CONTENT_CENTER => Self::CRABBY_UI_ALIGN_CONTENT_CENTER,
            crate::PonkoUiAlignContent::PONKO_UI_ALIGN_CONTENT_STRETCH => Self::CRABBY_UI_ALIGN_CONTENT_STRETCH,
            crate::PonkoUiAlignContent::PONKO_UI_ALIGN_CONTENT_SPACE_BETWEEN => {
                Self::CRABBY_UI_ALIGN_CONTENT_SPACE_BETWEEN
            }
            crate::PonkoUiAlignContent::PONKO_UI_ALIGN_CONTENT_SPACE_EVENLY => {
                Self::CRABBY_UI_ALIGN_CONTENT_SPACE_EVENLY
            }
            crate::PonkoUiAlignContent::PONKO_UI_ALIGN_CONTENT_SPACE_AROUND => {
                Self::CRABBY_UI_ALIGN_CONTENT_SPACE_AROUND
            }
        }
    }
}
impl From<CrabbyUiAlignContent> for crate::PonkoUiAlignContent {
    fn from(value: CrabbyUiAlignContent) -> Self {
        match value {
            CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_NORMAL => Self::PONKO_UI_ALIGN_CONTENT_NORMAL,
            CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_START => Self::PONKO_UI_ALIGN_CONTENT_START,
            CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_END => Self::PONKO_UI_ALIGN_CONTENT_END,
            CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_FLEX_START => Self::PONKO_UI_ALIGN_CONTENT_FLEX_START,
            CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_FLEX_END => Self::PONKO_UI_ALIGN_CONTENT_FLEX_END,
            CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_CENTER => Self::PONKO_UI_ALIGN_CONTENT_CENTER,
            CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_STRETCH => Self::PONKO_UI_ALIGN_CONTENT_STRETCH,
            CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_SPACE_BETWEEN => Self::PONKO_UI_ALIGN_CONTENT_SPACE_BETWEEN,
            CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_SPACE_EVENLY => Self::PONKO_UI_ALIGN_CONTENT_SPACE_EVENLY,
            CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_SPACE_AROUND => Self::PONKO_UI_ALIGN_CONTENT_SPACE_AROUND,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyUiAlignContent>() == std::mem::size_of::<crate::PonkoUiAlignContent>());
    assert!(std::mem::align_of::<CrabbyUiAlignContent>() == std::mem::align_of::<crate::PonkoUiAlignContent>());
    assert!(
        CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_NORMAL as u32
            == crate::PonkoUiAlignContent::PONKO_UI_ALIGN_CONTENT_NORMAL as u32
    );
    assert!(
        CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_START as u32
            == crate::PonkoUiAlignContent::PONKO_UI_ALIGN_CONTENT_START as u32
    );
    assert!(
        CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_END as u32
            == crate::PonkoUiAlignContent::PONKO_UI_ALIGN_CONTENT_END as u32
    );
    assert!(
        CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_FLEX_START as u32
            == crate::PonkoUiAlignContent::PONKO_UI_ALIGN_CONTENT_FLEX_START as u32
    );
    assert!(
        CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_FLEX_END as u32
            == crate::PonkoUiAlignContent::PONKO_UI_ALIGN_CONTENT_FLEX_END as u32
    );
    assert!(
        CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_CENTER as u32
            == crate::PonkoUiAlignContent::PONKO_UI_ALIGN_CONTENT_CENTER as u32
    );
    assert!(
        CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_STRETCH as u32
            == crate::PonkoUiAlignContent::PONKO_UI_ALIGN_CONTENT_STRETCH as u32
    );
    assert!(
        CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_SPACE_BETWEEN as u32
            == crate::PonkoUiAlignContent::PONKO_UI_ALIGN_CONTENT_SPACE_BETWEEN as u32
    );
    assert!(
        CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_SPACE_EVENLY as u32
            == crate::PonkoUiAlignContent::PONKO_UI_ALIGN_CONTENT_SPACE_EVENLY as u32
    );
    assert!(
        CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_SPACE_AROUND as u32
            == crate::PonkoUiAlignContent::PONKO_UI_ALIGN_CONTENT_SPACE_AROUND as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_CENTER;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_END;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_FLEX_END;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_FLEX_START;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_NORMAL;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_SPACE_AROUND;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_SPACE_BETWEEN;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_SPACE_EVENLY;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_START;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiAlignContent::CRABBY_UI_ALIGN_CONTENT_STRETCH;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiLengthKind {
    CRABBY_UI_LENGTH_AUTO,

    CRABBY_UI_LENGTH_POINTS,

    CRABBY_UI_LENGTH_PERCENT,
}
impl From<crate::PonkoUiLengthKind> for CrabbyUiLengthKind {
    fn from(value: crate::PonkoUiLengthKind) -> Self {
        match value {
            crate::PonkoUiLengthKind::PONKO_UI_LENGTH_AUTO => Self::CRABBY_UI_LENGTH_AUTO,
            crate::PonkoUiLengthKind::PONKO_UI_LENGTH_POINTS => Self::CRABBY_UI_LENGTH_POINTS,
            crate::PonkoUiLengthKind::PONKO_UI_LENGTH_PERCENT => Self::CRABBY_UI_LENGTH_PERCENT,
        }
    }
}
impl From<CrabbyUiLengthKind> for crate::PonkoUiLengthKind {
    fn from(value: CrabbyUiLengthKind) -> Self {
        match value {
            CrabbyUiLengthKind::CRABBY_UI_LENGTH_AUTO => Self::PONKO_UI_LENGTH_AUTO,
            CrabbyUiLengthKind::CRABBY_UI_LENGTH_POINTS => Self::PONKO_UI_LENGTH_POINTS,
            CrabbyUiLengthKind::CRABBY_UI_LENGTH_PERCENT => Self::PONKO_UI_LENGTH_PERCENT,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyUiLengthKind>() == std::mem::size_of::<crate::PonkoUiLengthKind>());
    assert!(std::mem::align_of::<CrabbyUiLengthKind>() == std::mem::align_of::<crate::PonkoUiLengthKind>());
    assert!(CrabbyUiLengthKind::CRABBY_UI_LENGTH_AUTO as u32 == crate::PonkoUiLengthKind::PONKO_UI_LENGTH_AUTO as u32);
    assert!(
        CrabbyUiLengthKind::CRABBY_UI_LENGTH_POINTS as u32 == crate::PonkoUiLengthKind::PONKO_UI_LENGTH_POINTS as u32
    );
    assert!(
        CrabbyUiLengthKind::CRABBY_UI_LENGTH_PERCENT as u32 == crate::PonkoUiLengthKind::PONKO_UI_LENGTH_PERCENT as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiLengthKind::CRABBY_UI_LENGTH_AUTO;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiLengthKind::CRABBY_UI_LENGTH_PERCENT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiLengthKind::CRABBY_UI_LENGTH_POINTS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiTrackKind {
    CRABBY_UI_TRACK_AUTO,

    CRABBY_UI_TRACK_POINTS,

    CRABBY_UI_TRACK_PERCENT,

    CRABBY_UI_TRACK_FR,

    CRABBY_UI_TRACK_MIN_CONTENT,

    CRABBY_UI_TRACK_MAX_CONTENT,
}
impl From<crate::PonkoUiTrackKind> for CrabbyUiTrackKind {
    fn from(value: crate::PonkoUiTrackKind) -> Self {
        match value {
            crate::PonkoUiTrackKind::PONKO_UI_TRACK_AUTO => Self::CRABBY_UI_TRACK_AUTO,
            crate::PonkoUiTrackKind::PONKO_UI_TRACK_POINTS => Self::CRABBY_UI_TRACK_POINTS,
            crate::PonkoUiTrackKind::PONKO_UI_TRACK_PERCENT => Self::CRABBY_UI_TRACK_PERCENT,
            crate::PonkoUiTrackKind::PONKO_UI_TRACK_FR => Self::CRABBY_UI_TRACK_FR,
            crate::PonkoUiTrackKind::PONKO_UI_TRACK_MIN_CONTENT => Self::CRABBY_UI_TRACK_MIN_CONTENT,
            crate::PonkoUiTrackKind::PONKO_UI_TRACK_MAX_CONTENT => Self::CRABBY_UI_TRACK_MAX_CONTENT,
        }
    }
}
impl From<CrabbyUiTrackKind> for crate::PonkoUiTrackKind {
    fn from(value: CrabbyUiTrackKind) -> Self {
        match value {
            CrabbyUiTrackKind::CRABBY_UI_TRACK_AUTO => Self::PONKO_UI_TRACK_AUTO,
            CrabbyUiTrackKind::CRABBY_UI_TRACK_POINTS => Self::PONKO_UI_TRACK_POINTS,
            CrabbyUiTrackKind::CRABBY_UI_TRACK_PERCENT => Self::PONKO_UI_TRACK_PERCENT,
            CrabbyUiTrackKind::CRABBY_UI_TRACK_FR => Self::PONKO_UI_TRACK_FR,
            CrabbyUiTrackKind::CRABBY_UI_TRACK_MIN_CONTENT => Self::PONKO_UI_TRACK_MIN_CONTENT,
            CrabbyUiTrackKind::CRABBY_UI_TRACK_MAX_CONTENT => Self::PONKO_UI_TRACK_MAX_CONTENT,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyUiTrackKind>() == std::mem::size_of::<crate::PonkoUiTrackKind>());
    assert!(std::mem::align_of::<CrabbyUiTrackKind>() == std::mem::align_of::<crate::PonkoUiTrackKind>());
    assert!(CrabbyUiTrackKind::CRABBY_UI_TRACK_AUTO as u32 == crate::PonkoUiTrackKind::PONKO_UI_TRACK_AUTO as u32);
    assert!(CrabbyUiTrackKind::CRABBY_UI_TRACK_POINTS as u32 == crate::PonkoUiTrackKind::PONKO_UI_TRACK_POINTS as u32);
    assert!(
        CrabbyUiTrackKind::CRABBY_UI_TRACK_PERCENT as u32 == crate::PonkoUiTrackKind::PONKO_UI_TRACK_PERCENT as u32
    );
    assert!(CrabbyUiTrackKind::CRABBY_UI_TRACK_FR as u32 == crate::PonkoUiTrackKind::PONKO_UI_TRACK_FR as u32);
    assert!(
        CrabbyUiTrackKind::CRABBY_UI_TRACK_MIN_CONTENT as u32
            == crate::PonkoUiTrackKind::PONKO_UI_TRACK_MIN_CONTENT as u32
    );
    assert!(
        CrabbyUiTrackKind::CRABBY_UI_TRACK_MAX_CONTENT as u32
            == crate::PonkoUiTrackKind::PONKO_UI_TRACK_MAX_CONTENT as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiTrackKind::CRABBY_UI_TRACK_AUTO;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiTrackKind::CRABBY_UI_TRACK_FR;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiTrackKind::CRABBY_UI_TRACK_MAX_CONTENT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiTrackKind::CRABBY_UI_TRACK_MIN_CONTENT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiTrackKind::CRABBY_UI_TRACK_PERCENT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiTrackKind::CRABBY_UI_TRACK_POINTS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiGridLineKind {
    CRABBY_UI_GRID_AUTO,

    CRABBY_UI_GRID_LINE,

    CRABBY_UI_GRID_SPAN,
}
impl From<crate::PonkoUiGridLineKind> for CrabbyUiGridLineKind {
    fn from(value: crate::PonkoUiGridLineKind) -> Self {
        match value {
            crate::PonkoUiGridLineKind::PONKO_UI_GRID_AUTO => Self::CRABBY_UI_GRID_AUTO,
            crate::PonkoUiGridLineKind::PONKO_UI_GRID_LINE => Self::CRABBY_UI_GRID_LINE,
            crate::PonkoUiGridLineKind::PONKO_UI_GRID_SPAN => Self::CRABBY_UI_GRID_SPAN,
        }
    }
}
impl From<CrabbyUiGridLineKind> for crate::PonkoUiGridLineKind {
    fn from(value: CrabbyUiGridLineKind) -> Self {
        match value {
            CrabbyUiGridLineKind::CRABBY_UI_GRID_AUTO => Self::PONKO_UI_GRID_AUTO,
            CrabbyUiGridLineKind::CRABBY_UI_GRID_LINE => Self::PONKO_UI_GRID_LINE,
            CrabbyUiGridLineKind::CRABBY_UI_GRID_SPAN => Self::PONKO_UI_GRID_SPAN,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyUiGridLineKind>() == std::mem::size_of::<crate::PonkoUiGridLineKind>());
    assert!(std::mem::align_of::<CrabbyUiGridLineKind>() == std::mem::align_of::<crate::PonkoUiGridLineKind>());
    assert!(CrabbyUiGridLineKind::CRABBY_UI_GRID_AUTO as u32 == crate::PonkoUiGridLineKind::PONKO_UI_GRID_AUTO as u32);
    assert!(CrabbyUiGridLineKind::CRABBY_UI_GRID_LINE as u32 == crate::PonkoUiGridLineKind::PONKO_UI_GRID_LINE as u32);
    assert!(CrabbyUiGridLineKind::CRABBY_UI_GRID_SPAN as u32 == crate::PonkoUiGridLineKind::PONKO_UI_GRID_SPAN as u32);
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiGridLineKind::CRABBY_UI_GRID_AUTO;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiGridLineKind::CRABBY_UI_GRID_LINE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiGridLineKind::CRABBY_UI_GRID_SPAN;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiTextAlign {
    CRABBY_UI_TEXT_ALIGN_START,

    CRABBY_UI_TEXT_ALIGN_CENTER,

    CRABBY_UI_TEXT_ALIGN_END,
}
impl From<crate::PonkoUiTextAlign> for CrabbyUiTextAlign {
    fn from(value: crate::PonkoUiTextAlign) -> Self {
        match value {
            crate::PonkoUiTextAlign::PONKO_UI_TEXT_ALIGN_START => Self::CRABBY_UI_TEXT_ALIGN_START,
            crate::PonkoUiTextAlign::PONKO_UI_TEXT_ALIGN_CENTER => Self::CRABBY_UI_TEXT_ALIGN_CENTER,
            crate::PonkoUiTextAlign::PONKO_UI_TEXT_ALIGN_END => Self::CRABBY_UI_TEXT_ALIGN_END,
        }
    }
}
impl From<CrabbyUiTextAlign> for crate::PonkoUiTextAlign {
    fn from(value: CrabbyUiTextAlign) -> Self {
        match value {
            CrabbyUiTextAlign::CRABBY_UI_TEXT_ALIGN_START => Self::PONKO_UI_TEXT_ALIGN_START,
            CrabbyUiTextAlign::CRABBY_UI_TEXT_ALIGN_CENTER => Self::PONKO_UI_TEXT_ALIGN_CENTER,
            CrabbyUiTextAlign::CRABBY_UI_TEXT_ALIGN_END => Self::PONKO_UI_TEXT_ALIGN_END,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyUiTextAlign>() == std::mem::size_of::<crate::PonkoUiTextAlign>());
    assert!(std::mem::align_of::<CrabbyUiTextAlign>() == std::mem::align_of::<crate::PonkoUiTextAlign>());
    assert!(
        CrabbyUiTextAlign::CRABBY_UI_TEXT_ALIGN_START as u32
            == crate::PonkoUiTextAlign::PONKO_UI_TEXT_ALIGN_START as u32
    );
    assert!(
        CrabbyUiTextAlign::CRABBY_UI_TEXT_ALIGN_CENTER as u32
            == crate::PonkoUiTextAlign::PONKO_UI_TEXT_ALIGN_CENTER as u32
    );
    assert!(
        CrabbyUiTextAlign::CRABBY_UI_TEXT_ALIGN_END as u32 == crate::PonkoUiTextAlign::PONKO_UI_TEXT_ALIGN_END as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiTextAlign::CRABBY_UI_TEXT_ALIGN_CENTER;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiTextAlign::CRABBY_UI_TEXT_ALIGN_END;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiTextAlign::CRABBY_UI_TEXT_ALIGN_START;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum CrabbyUiEventKind {
    #[default]
    CRABBY_UI_EVENT_CLICKED,

    CRABBY_UI_EVENT_CHANGED,

    CRABBY_UI_EVENT_SUBMITTED,

    CRABBY_UI_EVENT_SCREEN_POPPED,
}
impl From<crate::PonkoUiEventKind> for CrabbyUiEventKind {
    fn from(value: crate::PonkoUiEventKind) -> Self {
        match value {
            crate::PonkoUiEventKind::PONKO_UI_EVENT_CLICKED => Self::CRABBY_UI_EVENT_CLICKED,
            crate::PonkoUiEventKind::PONKO_UI_EVENT_CHANGED => Self::CRABBY_UI_EVENT_CHANGED,
            crate::PonkoUiEventKind::PONKO_UI_EVENT_SUBMITTED => Self::CRABBY_UI_EVENT_SUBMITTED,
            crate::PonkoUiEventKind::PONKO_UI_EVENT_SCREEN_POPPED => Self::CRABBY_UI_EVENT_SCREEN_POPPED,
        }
    }
}
impl From<CrabbyUiEventKind> for crate::PonkoUiEventKind {
    fn from(value: CrabbyUiEventKind) -> Self {
        match value {
            CrabbyUiEventKind::CRABBY_UI_EVENT_CLICKED => Self::PONKO_UI_EVENT_CLICKED,
            CrabbyUiEventKind::CRABBY_UI_EVENT_CHANGED => Self::PONKO_UI_EVENT_CHANGED,
            CrabbyUiEventKind::CRABBY_UI_EVENT_SUBMITTED => Self::PONKO_UI_EVENT_SUBMITTED,
            CrabbyUiEventKind::CRABBY_UI_EVENT_SCREEN_POPPED => Self::PONKO_UI_EVENT_SCREEN_POPPED,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyUiEventKind>() == std::mem::size_of::<crate::PonkoUiEventKind>());
    assert!(std::mem::align_of::<CrabbyUiEventKind>() == std::mem::align_of::<crate::PonkoUiEventKind>());
    assert!(
        CrabbyUiEventKind::CRABBY_UI_EVENT_CLICKED as u32 == crate::PonkoUiEventKind::PONKO_UI_EVENT_CLICKED as u32
    );
    assert!(
        CrabbyUiEventKind::CRABBY_UI_EVENT_CHANGED as u32 == crate::PonkoUiEventKind::PONKO_UI_EVENT_CHANGED as u32
    );
    assert!(
        CrabbyUiEventKind::CRABBY_UI_EVENT_SUBMITTED as u32 == crate::PonkoUiEventKind::PONKO_UI_EVENT_SUBMITTED as u32
    );
    assert!(
        CrabbyUiEventKind::CRABBY_UI_EVENT_SCREEN_POPPED as u32
            == crate::PonkoUiEventKind::PONKO_UI_EVENT_SCREEN_POPPED as u32
    );
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoUiGap as CrabbyUiGap;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoUiGridLine as CrabbyUiGridLine;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoUiLayout as CrabbyUiLayout;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoUiLength as CrabbyUiLength;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoUiPaint as CrabbyUiPaint;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoUiSides as CrabbyUiSides;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoUiTemplateTrack as CrabbyUiTemplateTrack;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoUiTrack as CrabbyUiTrack;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiEventKind::CRABBY_UI_EVENT_CHANGED;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiEventKind::CRABBY_UI_EVENT_CLICKED;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiEventKind::CRABBY_UI_EVENT_SCREEN_POPPED;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use CrabbyUiEventKind::CRABBY_UI_EVENT_SUBMITTED;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrabbyUiEvent {
    pub kind: CrabbyUiEventKind,

    pub id: u64,
}
impl From<crate::PonkoUiEvent> for CrabbyUiEvent {
    fn from(value: crate::PonkoUiEvent) -> Self {
        Self {
            kind: value.kind.into(),
            id: value.id,
        }
    }
}
impl From<CrabbyUiEvent> for crate::PonkoUiEvent {
    fn from(value: CrabbyUiEvent) -> Self {
        Self {
            kind: value.kind.into(),
            id: value.id,
        }
    }
}
const _: () = {
    assert!(std::mem::size_of::<CrabbyUiEvent>() == std::mem::size_of::<crate::PonkoUiEvent>());
    assert!(std::mem::align_of::<CrabbyUiEvent>() == std::mem::align_of::<crate::PonkoUiEvent>());
    assert!(std::mem::offset_of!(CrabbyUiEvent, kind) == std::mem::offset_of!(crate::PonkoUiEvent, kind));
    assert!(std::mem::offset_of!(CrabbyUiEvent, id) == std::mem::offset_of!(crate::PonkoUiEvent, id));
};
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoFrictionCurve as CrabbyFrictionCurve;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoUiInput as CrabbyUiInput;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoUiTheme as CrabbyUiTheme;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoVehicle3D as CrabbyVehicle3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PonkoWheel3D as CrabbyWheel3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_access_new() -> *mut CrabbyAccess {
    crate::ponko_access_new()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_access_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_access_free(access: *mut CrabbyAccess) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_access_free(access) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_access_read_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_access_read_component(access: *mut CrabbyAccess, name: *const c_char) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_access_read_component(access, name) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_access_write_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_access_write_component(access: *mut CrabbyAccess, name: *const c_char) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_access_write_component(access, name) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_access_read_resource`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_access_read_resource(access: *mut CrabbyAccess, name: *const c_char) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_access_read_resource(access, name) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_access_write_resource`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_access_write_resource(access: *mut CrabbyAccess, name: *const c_char) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_access_write_resource(access, name) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_access_read_events`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_access_read_events(access: *mut CrabbyAccess, name: *const c_char) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_access_read_events(access, name) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_access_send_events`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_access_send_events(access: *mut CrabbyAccess, name: *const c_char) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_access_send_events(access, name) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_access_commands`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_access_commands(access: *mut CrabbyAccess) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_access_commands(access) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_access_service`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_access_service(access: *mut CrabbyAccess, service: u32, write: bool) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_access_service(access, service, write) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_access_conflicts`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_access_conflicts(
    earlier: *const CrabbyAccess,
    later: *const CrabbyAccess,
    ordered: bool,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_access_conflicts(earlier, later, ordered, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_add_system_with_access`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_add_system_with_access(
    game: *mut CrabbyGame,
    phase_raw: u32,
    name: *const c_char,
    every: u32,
    access: *const CrabbyAccess,
    system_fn: CrabbySystemFn,
    user: *mut c_void,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_add_system_with_access(game, phase_raw, name, every, access, system_fn, user) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_access_report`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_access_report(
    game: *mut CrabbyGame,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_access_report(game, buffer, capacity, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_parallel_systems`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_parallel_systems(game: *mut CrabbyGame, on: bool) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_parallel_systems(game, on) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_schedule_plan`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_schedule_plan(
    game: *mut CrabbyGame,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_schedule_plan(game, buffer, capacity, out_len) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_action_map_new() -> *mut CrabbyActionMap {
    crate::ponko_action_map_new()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_action_map_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_action_map_free(map: *mut CrabbyActionMap) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_action_map_free(map) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_action_map_set`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_action_map_set(
    map: *mut CrabbyActionMap,
    name: *const c_char,
    kind: u32,
    bindings: *const CrabbyActionBinding,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_action_map_set(map, name, kind, bindings, count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_action_map_remove`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_action_map_remove(map: *mut CrabbyActionMap, name: *const c_char) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_action_map_remove(map, name) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_action_map_load`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_action_map_load(map: *mut CrabbyActionMap, json: *const c_char) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_action_map_load(map, json) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_action_map_save`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_action_map_save(
    map: *mut CrabbyActionMap,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_action_map_save(map, buffer, capacity, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_actions`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_actions(ctx: *mut CrabbyCtx, map: *mut CrabbyActionMap) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_actions(ctx, map) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_actions`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_actions(ctx: *mut CrabbyCtx, map: *mut CrabbyActionMap) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_actions(ctx, map) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_feed_input`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_feed_input(
    ctx: *mut CrabbyCtx,
    kind: u32,
    slot: u32,
    input: u32,
    value: f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_feed_input(ctx, kind, slot, input, value) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_action_down`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_action_down(
    ctx: *mut CrabbyCtx,
    name: *const c_char,
    value: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_action_down(ctx, name, value) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_action_pressed`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_action_pressed(
    ctx: *mut CrabbyCtx,
    name: *const c_char,
    value: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_action_pressed(ctx, name, value) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_action_released`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_action_released(
    ctx: *mut CrabbyCtx,
    name: *const c_char,
    value: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_action_released(ctx, name, value) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_action_axis`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_action_axis(
    ctx: *mut CrabbyCtx,
    name: *const c_char,
    value: *mut f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_action_axis(ctx, name, value) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_action_axis2`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_action_axis2(
    ctx: *mut CrabbyCtx,
    name: *const c_char,
    value: *mut CrabbyVec2,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_action_axis2(ctx, name, value) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_motion_clip`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_motion_clip(
    clip: *const CrabbyClip,
    out_motion: *mut *mut CrabbyMotion,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_motion_clip(clip, out_motion) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_motion_blend1d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_motion_blend1d(parameter: usize, out_motion: *mut *mut CrabbyMotion) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_motion_blend1d(parameter, out_motion) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_motion_blend2d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_motion_blend2d(x: usize, y: usize, out_motion: *mut *mut CrabbyMotion) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_motion_blend2d(x, y, out_motion) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_motion_blend1d_add`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_motion_blend1d_add(
    tree: *mut CrabbyMotion,
    threshold: f32,
    child: *mut CrabbyMotion,
    time_scale: f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_motion_blend1d_add(tree, threshold, child, time_scale) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_motion_blend2d_add`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_motion_blend2d_add(
    tree: *mut CrabbyMotion,
    x: f32,
    y: f32,
    child: *mut CrabbyMotion,
    time_scale: f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_motion_blend2d_add(tree, x, y, child, time_scale) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_motion_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_motion_free(motion: *mut CrabbyMotion) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_motion_free(motion) }
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_animation_graph_builder_new() -> *mut CrabbyAnimationGraphBuilder {
    crate::ponko_animation_graph_builder_new()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_animation_graph_builder_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_animation_graph_builder_free(builder: *mut CrabbyAnimationGraphBuilder) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_animation_graph_builder_free(builder) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_animation_graph_add_float`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_animation_graph_add_float(
    builder: *mut CrabbyAnimationGraphBuilder,
    value: f32,
    out_index: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_animation_graph_add_float(builder, value, out_index) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_animation_graph_add_bool`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_animation_graph_add_bool(
    builder: *mut CrabbyAnimationGraphBuilder,
    value: bool,
    out_index: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_animation_graph_add_bool(builder, value, out_index) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_animation_graph_add_trigger`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_animation_graph_add_trigger(
    builder: *mut CrabbyAnimationGraphBuilder,
    out_index: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_animation_graph_add_trigger(builder, out_index) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_animation_graph_add_state`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_animation_graph_add_state(
    builder_ptr: *mut CrabbyAnimationGraphBuilder,
    motion: *mut CrabbyMotion,
    speed: f32,
    speed_parameter: i64,
    looping: bool,
    out_index: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_animation_graph_add_state(builder_ptr, motion, speed, speed_parameter, looping, out_index) })
        .into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_animation_graph_add_event`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_animation_graph_add_event(
    builder_ptr: *mut CrabbyAnimationGraphBuilder,
    state: usize,
    time: f32,
    name: *const c_char,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_animation_graph_add_event(builder_ptr, state, time, name) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_animation_graph_add_transition`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_animation_graph_add_transition(
    builder_ptr: *mut CrabbyAnimationGraphBuilder,
    from: i64,
    to: usize,
    exit_time: f32,
    duration: f32,
    offset: f32,
    out_index: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe {
        crate::ponko_animation_graph_add_transition(builder_ptr, from, to, exit_time, duration, offset, out_index)
    })
    .into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_animation_graph_add_condition`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_animation_graph_add_condition(
    builder_ptr: *mut CrabbyAnimationGraphBuilder,
    transition: usize,
    kind: CrabbyAnimationCondition,
    parameter: usize,
    value: f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_animation_graph_add_condition(builder_ptr, transition, kind.into(), parameter, value) })
        .into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_animation_graph_build`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_animation_graph_build(
    builder_ptr: *mut CrabbyAnimationGraphBuilder,
    initial: usize,
    out_graph: *mut *mut CrabbyAnimationGraph,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_animation_graph_build(builder_ptr, initial, out_graph) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_animation_graph_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_animation_graph_free(graph: *mut CrabbyAnimationGraph) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_animation_graph_free(graph) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_animator`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_animator(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    graph: *const CrabbyAnimationGraph,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_animator(ctx, entity_bits, graph) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_animator_set_float`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_animator_set_float(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    parameter: usize,
    value: f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_animator_set_float(ctx, entity_bits, parameter, value) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_animator_set_bool`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_animator_set_bool(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    parameter: usize,
    value: bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_animator_set_bool(ctx, entity_bits, parameter, value) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_animator_set_trigger`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_animator_set_trigger(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    parameter: usize,
    value: bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_animator_set_trigger(ctx, entity_bits, parameter, value) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_animator_state`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_animator_state(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    out_state: *mut usize,
    out_normalized_time: *mut f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_animator_state(ctx, entity_bits, out_state, out_normalized_time) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_animator_crossfade`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_animator_crossfade(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    state: usize,
    duration: f32,
    offset: f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_animator_crossfade(ctx, entity_bits, state, duration, offset) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_animator_set_manual`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_animator_set_manual(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    manual: bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_animator_set_manual(ctx, entity_bits, manual) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_animator_update`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_animator_update(ctx: *mut CrabbyCtx, entity_bits: u64, dt: f32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_animator_update(ctx, entity_bits, dt) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_animator_next_event`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_animator_next_event(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    buffer: *mut c_char,
    capacity: usize,
    out_found: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_animator_next_event(ctx, entity_bits, buffer, capacity, out_found) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_animator_add_layer`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_animator_add_layer(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    graph: *const CrabbyAnimationGraph,
    mode: u32,
    mask_joint: *const c_char,
    out_layer: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_animator_add_layer(ctx, entity_bits, graph, mode, mask_joint, out_layer) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_animator_set_layer_weight`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_animator_set_layer_weight(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    layer: usize,
    weight: f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_animator_set_layer_weight(ctx, entity_bits, layer, weight) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_animator_layer_set_float`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_animator_layer_set_float(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    layer_index: usize,
    parameter: usize,
    value: f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_animator_layer_set_float(ctx, entity_bits, layer_index, parameter, value) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_animator_layer_set_bool`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_animator_layer_set_bool(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    layer_index: usize,
    parameter: usize,
    value: bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_animator_layer_set_bool(ctx, entity_bits, layer_index, parameter, value) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_animator_layer_set_trigger`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_animator_layer_set_trigger(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    layer_index: usize,
    parameter: usize,
    value: bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_animator_layer_set_trigger(ctx, entity_bits, layer_index, parameter, value) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_animator_layer_crossfade`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_animator_layer_crossfade(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    layer_index: usize,
    state: usize,
    duration: f32,
    offset: f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_animator_layer_crossfade(ctx, entity_bits, layer_index, state, duration, offset) })
        .into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_animator_set_root_motion`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_animator_set_root_motion(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    mode: u32,
    root_joint: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_animator_set_root_motion(ctx, entity_bits, mode, root_joint) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_animator_root_motion`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_animator_root_motion(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    out_position: *mut CrabbyVec3,
    out_rotation: *mut CrabbyQuat,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_animator_root_motion(ctx, entity_bits, out_position, out_rotation) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_attached_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_attached_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_attached_component(world, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_follow_attachments`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_follow_attachments(game: *mut CrabbyGame) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_follow_attachments(game) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_audio_effects_default() -> CrabbyAudioEffects {
    crate::ponko_audio_effects_default()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_audio_settings3d_default() -> CrabbyAudioSettings3D {
    crate::ponko_audio_settings3d_default().into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_audio_bus`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_audio_bus(
    ctx: *mut CrabbyCtx,
    name: *const c_char,
    out_bus: *mut CrabbyAudioBus,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_audio_bus(ctx, name, out_bus) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_audio_bus_volume`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_audio_bus_volume(
    ctx: *mut CrabbyCtx,
    bus: CrabbyAudioBus,
    volume: f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_audio_bus_volume(ctx, bus, volume) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_mute_audio_bus`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_mute_audio_bus(
    ctx: *mut CrabbyCtx,
    bus: CrabbyAudioBus,
    muted: bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_mute_audio_bus(ctx, bus, muted) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_play_sound_on`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_play_sound_on(
    ctx: *mut CrabbyCtx,
    sound: CrabbySound,
    params: *const CrabbySoundParams,
    bus: CrabbyAudioBus,
    effects: *const CrabbyAudioEffects,
    out_playing: *mut CrabbyPlaying,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_play_sound_on(ctx, sound, params, bus, effects, out_playing) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_sound_effects`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_sound_effects(
    ctx: *mut CrabbyCtx,
    playing: CrabbyPlaying,
    effects: *const CrabbyAudioEffects,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_sound_effects(ctx, playing, effects) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_load_stream`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_stream(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    out_sound: *mut CrabbySound,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_load_stream(ctx, path, out_sound) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_play_sound_at`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_play_sound_at(
    ctx: *mut CrabbyCtx,
    sound: CrabbySound,
    position: CrabbyVec3,
    settings: *const CrabbyAudioSettings3D,
    out_playing: *mut CrabbyPlaying,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_play_sound_at(ctx, sound, position, settings.cast(), out_playing) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_audio_source3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_audio_source3d(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    sound: CrabbySound,
    settings: *const CrabbyAudioSettings3D,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_audio_source3d(ctx, entity_bits, sound, settings.cast()) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_remove_audio_source3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_remove_audio_source3d(ctx: *mut CrabbyCtx, entity_bits: u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_remove_audio_source3d(ctx, entity_bits) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_audio_listener`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_audio_listener(
    ctx: *mut CrabbyCtx,
    listener: *const CrabbyAudioListener3D,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_audio_listener(ctx, listener) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_audio_listener`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_audio_listener(
    ctx: *mut CrabbyCtx,
    out_listener: *mut CrabbyAudioListener3D,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_audio_listener(ctx, out_listener) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_SINCE_LAST_RUN as CRABBY_SINCE_LAST_RUN;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_query_options_default() -> CrabbyQueryOptions {
    crate::ponko_query_options_default()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_component_track_changes`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_component_track_changes(world: *mut CrabbyWorld, id: u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_component_track_changes(world, id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_world_change_tick`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_world_change_tick(world: *mut CrabbyWorld, out_tick: *mut u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_world_change_tick(world, out_tick) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_world_advance_change_tick`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_world_advance_change_tick(world: *mut CrabbyWorld, out_tick: *mut u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_world_advance_change_tick(world, out_tick) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_world_changes_since`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_world_changes_since(world: *mut CrabbyWorld, out_tick: *mut u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_world_changes_since(world, out_tick) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_world_set_changes_since`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_world_set_changes_since(world: *mut CrabbyWorld, tick: u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_world_set_changes_since(world, tick) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_mark_changed`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_mark_changed(world: *mut CrabbyWorld, entity: u64, id: u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_mark_changed(world, entity, id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_removed`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_removed(
    world: *mut CrabbyWorld,
    id: u32,
    since: u64,
    out_entities: *mut u64,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_removed(world, id, since, out_entities, capacity, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_query_with`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_query_with(
    world: *mut CrabbyWorld,
    components: *const u32,
    count: usize,
    options: *const CrabbyQueryOptions,
    callback: CrabbyQueryFn,
    user: *mut c_void,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_query_with(world, components, count, options, callback, user) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_contact_modifier3d_default() -> CrabbyContactModifier3D {
    crate::ponko_contact_modifier3d_default()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_contact_modifier3d_one_way(up: CrabbyVec3) -> CrabbyContactModifier3D {
    crate::ponko_contact_modifier3d_one_way(up)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_contact_modifier3d_conveyor(velocity: CrabbyVec3) -> CrabbyContactModifier3D {
    crate::ponko_contact_modifier3d_conveyor(velocity)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_contact_modifier3d_hook() -> CrabbyContactModifier3D {
    crate::ponko_contact_modifier3d_hook()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_contact_modifier3d_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_contact_modifier3d_component(
    world: *mut CrabbyWorld,
    out_id: *mut u32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_contact_modifier3d_component(world, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_contact_hook3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_contact_hook3d(
    ctx: *mut CrabbyCtx,
    hook: CrabbyContactHook3D,
    user: *mut c_void,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_contact_hook3d(ctx, hook, user) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_contact_pair3d_entities`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_contact_pair3d_entities(
    pair: *mut CrabbyContactPair3D,
    out_a: *mut u64,
    out_b: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_contact_pair3d_entities(pair, out_a, out_b) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_contact_pair3d_normal`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_contact_pair3d_normal(
    pair: *mut CrabbyContactPair3D,
    out_normal: *mut CrabbyVec3,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_contact_pair3d_normal(pair, out_normal) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_contact_pair3d_point_count`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_contact_pair3d_point_count(
    pair: *mut CrabbyContactPair3D,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_contact_pair3d_point_count(pair, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_contact_pair3d_point`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_contact_pair3d_point(
    pair: *mut CrabbyContactPair3D,
    index: usize,
    out_point: *mut CrabbyVec3,
    out_distance: *mut f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_contact_pair3d_point(pair, index, out_point, out_distance) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_contact_pair3d_ignore`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_contact_pair3d_ignore(pair: *mut CrabbyContactPair3D) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_contact_pair3d_ignore(pair) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_contact_pair3d_remove_point`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_contact_pair3d_remove_point(
    pair: *mut CrabbyContactPair3D,
    index: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_contact_pair3d_remove_point(pair, index) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_contact_pair3d_friction`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_contact_pair3d_friction(
    pair: *mut CrabbyContactPair3D,
    out_friction: *mut f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_contact_pair3d_friction(pair, out_friction) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_contact_pair3d_set_friction`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_contact_pair3d_set_friction(
    pair: *mut CrabbyContactPair3D,
    friction: f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_contact_pair3d_set_friction(pair, friction) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_contact_pair3d_restitution`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_contact_pair3d_restitution(
    pair: *mut CrabbyContactPair3D,
    out_restitution: *mut f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_contact_pair3d_restitution(pair, out_restitution) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_contact_pair3d_set_restitution`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_contact_pair3d_set_restitution(
    pair: *mut CrabbyContactPair3D,
    restitution: f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_contact_pair3d_set_restitution(pair, restitution) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_contact_pair3d_set_surface_velocity`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_contact_pair3d_set_surface_velocity(
    pair: *mut CrabbyContactPair3D,
    velocity: CrabbyVec3,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_contact_pair3d_set_surface_velocity(pair, velocity) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_dt`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_dt(ctx: *const CrabbyCtx) -> f32 {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_ctx_dt(ctx) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_sim_tick`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_sim_tick(ctx: *const CrabbyCtx) -> u64 {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_ctx_sim_tick(ctx) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_sim_time`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_sim_time(ctx: *const CrabbyCtx) -> f64 {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_ctx_sim_time(ctx) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_time`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_time(ctx: *const CrabbyCtx) -> f64 {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_ctx_time(ctx) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_frame`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_frame(ctx: *const CrabbyCtx) -> u64 {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_ctx_frame(ctx) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_fps`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_fps(ctx: *const CrabbyCtx) -> f32 {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_ctx_fps(ctx) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_random_bits`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_random_bits(ctx: *const CrabbyCtx) -> u64 {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_ctx_random_bits(ctx) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_window_size`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_window_size(ctx: *const CrabbyCtx) -> CrabbyVec2 {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_ctx_window_size(ctx) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_world`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_world(ctx: *mut CrabbyCtx) -> *mut CrabbyWorld {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_ctx_world(ctx) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_quit`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_quit(ctx: *mut CrabbyCtx) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_quit(ctx) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_pause`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_pause(ctx: *mut CrabbyCtx) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_pause(ctx) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_resume`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_resume(ctx: *mut CrabbyCtx) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_resume(ctx) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_step`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_step(ctx: *mut CrabbyCtx) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_step(ctx) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_sim_speed`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_sim_speed(ctx: *mut CrabbyCtx, speed: f32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_sim_speed(ctx, speed) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_sim_speed`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_sim_speed(ctx: *mut CrabbyCtx, out_speed: *mut f32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_sim_speed(ctx, out_speed) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_is_paused`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_is_paused(ctx: *mut CrabbyCtx, out_paused: *mut bool) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_is_paused(ctx, out_paused) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_key_down`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_key_down(ctx: *mut CrabbyCtx, input: u32, out_state: *mut bool) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_key_down(ctx, input, out_state) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_key_pressed`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_key_pressed(ctx: *mut CrabbyCtx, input: u32, out_state: *mut bool) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_key_pressed(ctx, input, out_state) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_key_released`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_key_released(
    ctx: *mut CrabbyCtx,
    input: u32,
    out_state: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_key_released(ctx, input, out_state) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_mouse_down`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_mouse_down(ctx: *mut CrabbyCtx, input: u32, out_state: *mut bool) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_mouse_down(ctx, input, out_state) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_mouse_pressed`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_mouse_pressed(
    ctx: *mut CrabbyCtx,
    input: u32,
    out_state: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_mouse_pressed(ctx, input, out_state) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_mouse_released`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_mouse_released(
    ctx: *mut CrabbyCtx,
    input: u32,
    out_state: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_mouse_released(ctx, input, out_state) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_gamepad_count`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_gamepad_count(ctx: *mut CrabbyCtx, out_count: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_gamepad_count(ctx, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_gamepad_at`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_gamepad_at(ctx: *mut CrabbyCtx, index: u32, out_pad: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_gamepad_at(ctx, index, out_pad) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_gamepad_down`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_gamepad_down(
    ctx: *mut CrabbyCtx,
    pad: u32,
    button: u32,
    out_state: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_gamepad_down(ctx, pad, button, out_state) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_gamepad_pressed`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_gamepad_pressed(
    ctx: *mut CrabbyCtx,
    pad: u32,
    button: u32,
    out_state: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_gamepad_pressed(ctx, pad, button, out_state) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_gamepad_released`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_gamepad_released(
    ctx: *mut CrabbyCtx,
    pad: u32,
    button: u32,
    out_state: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_gamepad_released(ctx, pad, button, out_state) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_gamepad_axis`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_gamepad_axis(
    ctx: *mut CrabbyCtx,
    pad: u32,
    axis: u32,
    out_value: *mut f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_gamepad_axis(ctx, pad, axis, out_value) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_mouse_pos`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_mouse_pos(ctx: *mut CrabbyCtx, out_pos: *mut CrabbyVec2) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_mouse_pos(ctx, out_pos) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_mouse_world`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_mouse_world(ctx: *mut CrabbyCtx, out_pos: *mut CrabbyVec2) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_mouse_world(ctx, out_pos) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_mouse_wheel`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_mouse_wheel(ctx: *mut CrabbyCtx, out_wheel: *mut CrabbyVec2) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_mouse_wheel(ctx, out_wheel) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_mouse_motion`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_mouse_motion(ctx: *mut CrabbyCtx, out_motion: *mut CrabbyVec2) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_mouse_motion(ctx, out_motion) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_cursor_locked`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_cursor_locked(ctx: *mut CrabbyCtx, locked: bool) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_cursor_locked(ctx, locked) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_cursor_locked`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_cursor_locked(ctx: *mut CrabbyCtx, out_locked: *mut bool) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_cursor_locked(ctx, out_locked) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_load_texture`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_texture(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    out_texture: *mut CrabbyTexture,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_load_texture(ctx, path, out_texture) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_load_json`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_json(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    entity: u64,
    component: u32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_load_json(ctx, path, entity, component) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_load_resource_json`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_resource_json(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    name: *const c_char,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_load_resource_json(ctx, path, name) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_was_reloaded`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_was_reloaded(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    out_reloaded: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_was_reloaded(ctx, path, out_reloaded) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_create_texture`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_texture(
    ctx: *mut CrabbyCtx,
    width: u32,
    height: u32,
    rgba: *const u8,
    len: usize,
    out_texture: *mut CrabbyTexture,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_create_texture(ctx, width, height, rgba, len, out_texture) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_update_texture`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_update_texture(
    ctx: *mut CrabbyCtx,
    texture: CrabbyTexture,
    rgba: *const u8,
    len: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_update_texture(ctx, texture, rgba, len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_clear`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_clear(ctx: *mut CrabbyCtx, color: CrabbyColor) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_clear(ctx, color) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_draw`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_draw(ctx: *mut CrabbyCtx, texture: CrabbyTexture, pos: CrabbyVec2) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_draw(ctx, texture, pos) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_draw_ex`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_draw_ex(
    ctx: *mut CrabbyCtx,
    texture: CrabbyTexture,
    pos: CrabbyVec2,
    params: *const CrabbyDrawParams,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_draw_ex(ctx, texture, pos, params) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_draw_text`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_draw_text(
    ctx: *mut CrabbyCtx,
    string: *const c_char,
    pos: CrabbyVec2,
    size: f32,
    color: CrabbyColor,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_draw_text(ctx, string, pos, size, color) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_measure_text`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_measure_text(
    ctx: *mut CrabbyCtx,
    string: *const c_char,
    size: f32,
    out_size: *mut CrabbyVec2,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_measure_text(ctx, string, size, out_size) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_camera`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_camera(ctx: *mut CrabbyCtx, out_camera: *mut CrabbyCamera) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_camera(ctx, out_camera) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_camera`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_camera(ctx: *mut CrabbyCtx, camera: *const CrabbyCamera) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_camera(ctx, camera) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_despawn_later`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_despawn_later(ctx: *mut CrabbyCtx, entity: u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_despawn_later(ctx, entity) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_spawn_later`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_spawn_later(
    ctx: *mut CrabbyCtx,
    values: *const CrabbyValue,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_spawn_later(ctx, values, count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_later`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_later(
    ctx: *mut CrabbyCtx,
    entity: u64,
    value: *const CrabbyValue,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_later(ctx, entity, value) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_remove_later`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_remove_later(ctx: *mut CrabbyCtx, entity: u64, component: u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_remove_later(ctx, entity, component) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_send_event`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_send_event(
    ctx: *mut CrabbyCtx,
    type_id: u32,
    data: *const c_void,
    size: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_send_event(ctx, type_id, data, size) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_read_events`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_read_events(
    ctx: *mut CrabbyCtx,
    type_id: u32,
    size: usize,
    out_data: *mut *const c_void,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_read_events(ctx, type_id, size, out_data, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_spatial_radius`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_spatial_radius(
    ctx: *const CrabbyCtx,
    center: CrabbyVec2,
    radius: f32,
    out_hits: *mut CrabbyGridHit,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_spatial_radius(ctx, center, radius, out_hits, capacity, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_spatial_rect`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_spatial_rect(
    ctx: *const CrabbyCtx,
    min: CrabbyVec2,
    max: CrabbyVec2,
    out_hits: *mut CrabbyGridHit,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_spatial_rect(ctx, min, max, out_hits, capacity, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_spatial_nearest`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_spatial_nearest(
    ctx: *const CrabbyCtx,
    center: CrabbyVec2,
    max_radius: f32,
    exclude: u64,
    out_hit: *mut CrabbyGridHit,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_spatial_nearest(ctx, center, max_radius, exclude, out_hit) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_spatial_view`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_spatial_view(
    ctx: *const CrabbyCtx,
    out_view: *mut crate::CrabbyGridView,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_spatial_view(ctx, out_view) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_cast_ray_filtered`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_cast_ray_filtered(
    ctx: *mut CrabbyCtx,
    origin: CrabbyVec2,
    direction: CrabbyVec2,
    max_distance: f32,
    filter: CrabbyQueryFilter2D,
    out_found: *mut bool,
    out_hit: *mut CrabbyRayHit,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_cast_ray_filtered(ctx, origin, direction, max_distance, filter, out_found, out_hit) })
        .into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_overlap_ball`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_overlap_ball(
    ctx: *mut CrabbyCtx,
    center: CrabbyVec2,
    radius: f32,
    filter: CrabbyQueryFilter2D,
    out_entities: *mut CrabbyEntity,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_overlap_ball(ctx, center, radius, filter, out_entities, capacity, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_spawn_prefab3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_spawn_prefab3d(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    at: CrabbyVec3,
    output: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_spawn_prefab3d(ctx, path, at, output) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_spawn_prefab_transform3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_spawn_prefab_transform3d(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    at: CrabbyTransform3D,
    output: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_spawn_prefab_transform3d(ctx, path, at, output) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_spawn_prefab_transform2d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_spawn_prefab_transform2d(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    at: CrabbyTransform2D,
    output: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_spawn_prefab_transform2d(ctx, path, at, output) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_filter_with(component: u32) -> *mut CrabbyFilter {
    crate::ponko_filter_with(component)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_filter_without(component: u32) -> *mut CrabbyFilter {
    crate::ponko_filter_without(component)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_filter_always() -> *mut CrabbyFilter {
    crate::ponko_filter_always()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_filter_never() -> *mut CrabbyFilter {
    crate::ponko_filter_never()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_filter_all`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_filter_all(parts: *const *const CrabbyFilter, count: usize) -> *mut CrabbyFilter {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_filter_all(parts, count) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_filter_any`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_filter_any(parts: *const *const CrabbyFilter, count: usize) -> *mut CrabbyFilter {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_filter_any(parts, count) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_filter_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_filter_free(filter: *mut CrabbyFilter) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_filter_free(filter) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_filter_check`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_filter_check(world: *mut CrabbyWorld, filter: *const CrabbyFilter) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_filter_check(world, filter) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_query_filtered`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_query_filtered(
    world: *mut CrabbyWorld,
    components: *const u32,
    count: usize,
    filter: *const CrabbyFilter,
    options: *const CrabbyQueryOptions,
    callback: CrabbyQueryFn,
    user: *mut c_void,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_query_filtered(world, components, count, filter, options, callback, user) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_NAV_WALL as CRABBY_NAV_WALL;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_nav_new(width: u32, height: u32, cell_size: f32) -> *mut CrabbyNavGrid {
    crate::ponko_nav_new(width, height, cell_size)
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_nav_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_nav_free(nav: *mut CrabbyNavGrid) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_nav_free(nav) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_nav_set_origin`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_nav_set_origin(nav: *mut CrabbyNavGrid, origin: CrabbyVec2) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_nav_set_origin(nav, origin) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_nav_costs`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_nav_costs(
    nav: *mut CrabbyNavGrid,
    out_costs: *mut *mut u8,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_nav_costs(nav, out_costs, out_len) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_flow_new() -> *mut CrabbyFlowField {
    crate::ponko_flow_new()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_flow_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_flow_free(flow: *mut CrabbyFlowField) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_flow_free(flow) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_flow_compute`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_flow_compute(
    flow: *mut CrabbyFlowField,
    nav: *const CrabbyNavGrid,
    goals: *const CrabbyVec2,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_flow_compute(flow, nav, goals, count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_flow_direction`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_flow_direction(flow: *const CrabbyFlowField, pos: CrabbyVec2) -> CrabbyVec2 {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_flow_direction(flow, pos) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_flow_distance`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_flow_distance(
    flow: *const CrabbyFlowField,
    pos: CrabbyVec2,
    out_distance: *mut f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_flow_distance(flow, pos, out_distance) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_flow_view`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_flow_view(flow: *const CrabbyFlowField, out_view: *mut CrabbyFlowView) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_flow_view(flow, out_view) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_SYSTEM_ANIMATE_SPRITES as CRABBY_SYSTEM_ANIMATE_SPRITES;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_SYSTEM_DRAW_SPRITES as CRABBY_SYSTEM_DRAW_SPRITES;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_SYSTEM_DRAW_TILEMAPS as CRABBY_SYSTEM_DRAW_TILEMAPS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_SYSTEM_FOLLOW_ATTACHMENTS as CRABBY_SYSTEM_FOLLOW_ATTACHMENTS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_SYSTEM_INTERPOLATE_PHYSICS3D as CRABBY_SYSTEM_INTERPOLATE_PHYSICS3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_SYSTEM_PHYSICS as CRABBY_SYSTEM_PHYSICS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_SYSTEM_PHYSICS3D as CRABBY_SYSTEM_PHYSICS3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_SYSTEM_REBUILD_SPATIAL_GRID as CRABBY_SYSTEM_REBUILD_SPATIAL_GRID;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_SYSTEM_STEP_NAVMESH3D as CRABBY_SYSTEM_STEP_NAVMESH3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_SYSTEM_UPDATE_ANIMATORS as CRABBY_SYSTEM_UPDATE_ANIMATORS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_SYSTEM_UPDATE_AUDIO3D as CRABBY_SYSTEM_UPDATE_AUDIO3D;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_SYSTEM_UPDATE_PARTICLES as CRABBY_SYSTEM_UPDATE_PARTICLES;
/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_new`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_new(title: *const c_char, width: u32, height: u32) -> *mut CrabbyGame {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_game_new(title, width, height) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_free(game: *mut CrabbyGame) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_game_free(game) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_fixed_hz`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_fixed_hz(game: *mut CrabbyGame, hz: u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_fixed_hz(game, hz) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_seed`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_seed(game: *mut CrabbyGame, seed: u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_seed(game, seed) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_vsync`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_vsync(game: *mut CrabbyGame, on: bool) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_vsync(game, on) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_msaa`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_msaa(game: *mut CrabbyGame, samples: u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_msaa(game, samples) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_fullscreen`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_fullscreen(game: *mut CrabbyGame, on: bool) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_fullscreen(game, on) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_spatial_grid`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_spatial_grid(game: *mut CrabbyGame, cell_size: f32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_spatial_grid(game, cell_size) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_camera_controls`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_camera_controls(game: *mut CrabbyGame) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_camera_controls(game) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_control_endpoint`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_control_endpoint(game: *mut CrabbyGame, on: bool) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_control_endpoint(game, on) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_hot_reload`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_hot_reload(game: *mut CrabbyGame, on: bool) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_hot_reload(game, on) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_inspector`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_inspector(game: *mut CrabbyGame, on: bool) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_inspector(game, on) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_sim_system`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_sim_system(
    game: *mut CrabbyGame,
    name: *const c_char,
    every: u32,
    system_fn: CrabbySystemFn,
    user: *mut c_void,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_sim_system(game, name, every, system_fn, user) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_add_system`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_add_system(
    game: *mut CrabbyGame,
    phase: u32,
    name: *const c_char,
    every: u32,
    system_fn: CrabbySystemFn,
    user: *mut c_void,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_add_system(game, phase, name, every, system_fn, user) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_add_system_id`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_add_system_id(
    game: *mut CrabbyGame,
    phase: u32,
    name: *const c_char,
    every: u32,
    system_fn: CrabbySystemFn,
    user: *mut c_void,
    out_id: *mut u32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_add_system_id(game, phase, name, every, system_fn, user, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_remove`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_remove(game: *mut CrabbyGame, system_id: u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_remove(game, system_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_setup`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_setup(
    game: *mut CrabbyGame,
    setup: CrabbySystemFn,
    user: *mut c_void,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_setup(game, setup, user) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_plugin`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_plugin(
    game: *mut CrabbyGame,
    build_fn: CrabbyPluginBuildFn,
    setup: CrabbySystemFn,
    user: *mut c_void,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_plugin(game, build_fn, setup, user) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_late_system`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_late_system(
    game: *mut CrabbyGame,
    name: *const c_char,
    system_fn: CrabbySystemFn,
    user: *mut c_void,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_late_system(game, name, system_fn, user) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_system`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_system(
    game: *mut CrabbyGame,
    name: *const c_char,
    system_fn: CrabbySystemFn,
    user: *mut c_void,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_system(game, name, system_fn, user) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_remove_system`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_remove_system(game: *mut CrabbyGame, name_ptr: *const c_char) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_remove_system(game, name_ptr) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_profile_headless`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_profile_headless(
    game: *mut CrabbyGame,
    warmup: u64,
    ticks: u64,
    setup: CrabbySystemFn,
    user: *mut c_void,
    out_world: *mut *mut CrabbyWorld,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_profile_headless(game, warmup, ticks, setup, user, out_world) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_run`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_run(
    game: *mut CrabbyGame,
    setup: CrabbySystemFn,
    user: *mut c_void,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_run(game, setup, user) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_run_headless`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_run_headless(
    game: *mut CrabbyGame,
    ticks: u64,
    setup: CrabbySystemFn,
    user: *mut c_void,
    out_world: *mut *mut CrabbyWorld,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_run_headless(game, ticks, setup, user, out_world) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_gpu_capabilities`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_gpu_capabilities(
    ctx: *const CrabbyCtx,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_gpu_capabilities(ctx, buffer, capacity, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_gpu_tier`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_gpu_tier(ctx: *const CrabbyCtx, out_tier: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_gpu_tier(ctx, out_tier) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_gpu_feature`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_gpu_feature(
    ctx: *const CrabbyCtx,
    feature_id: u32,
    out_available: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_gpu_feature(ctx, feature_id, out_available) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_gpu_tier_report`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_gpu_tier_report(
    tier_id: u32,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_gpu_tier_report(tier_id, buffer, capacity, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_gpu_tier_feature`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_gpu_tier_feature(
    tier_id: u32,
    feature_id: u32,
    out_available: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_gpu_tier_feature(tier_id, feature_id, out_available) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_graphics_quality_for_tier`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_graphics_quality_for_tier(
    tier_id: u32,
    out_quality: *mut CrabbyGraphicsQuality,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_graphics_quality_for_tier(tier_id, out_quality) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_recommended_graphics_quality`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_recommended_graphics_quality(
    ctx: *const CrabbyCtx,
    out_quality: *mut CrabbyGraphicsQuality,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_recommended_graphics_quality(ctx, out_quality) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_grid_new(cell_size: f32) -> *mut CrabbyGrid {
    crate::ponko_grid_new(cell_size)
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_grid_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_grid_free(grid: *mut CrabbyGrid) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_grid_free(grid) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_grid_rebuild`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_grid_rebuild(
    grid: *mut CrabbyGrid,
    items: *const CrabbyGridItem,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_grid_rebuild(grid, items, count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_grid_radius`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_grid_radius(
    grid: *const CrabbyGrid,
    center: CrabbyVec2,
    radius: f32,
    out_items: *mut CrabbyGridItem,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_grid_radius(grid, center, radius, out_items, capacity, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_grid_len`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_grid_len(grid: *const CrabbyGrid) -> usize {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_grid_len(grid) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_grid_view`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_grid_view(grid: *const CrabbyGrid, out_view: *mut CrabbyGridView) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_grid_view(grid, out_view) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_HANDLE_NONE as CRABBY_HANDLE_NONE;
/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_handle_is_live`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_handle_is_live(
    ctx: *mut CrabbyCtx,
    kind: u32,
    id: u32,
    out_live: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_handle_is_live(ctx, kind, id, out_live) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_handle_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_handle_free(ctx: *mut CrabbyCtx, kind: u32, id: u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_handle_free(ctx, kind, id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_live_counts`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_live_counts(
    ctx: *mut CrabbyCtx,
    out_counts: *mut CrabbyLiveCounts,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_live_counts(ctx, out_counts) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_asset_count`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_asset_count(
    ctx: *mut CrabbyCtx,
    asset_class_id: u32,
    out_count: *mut CrabbyAssetCount,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_asset_count(ctx, asset_class_id, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_asset_handles`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_asset_handles(
    ctx: *mut CrabbyCtx,
    asset_class_id: u32,
    records: *mut CrabbyAssetRecord,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_asset_handles(ctx, asset_class_id, records, capacity, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_asset_owner`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_asset_owner(
    ctx: *mut CrabbyCtx,
    asset_class_id: u32,
    id: u32,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_asset_owner(ctx, asset_class_id, id, buffer, capacity, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_unload_mesh`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_unload_mesh(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    out_unloaded: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_unload_mesh(ctx, path, out_unloaded) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_MAX_GAMEPADS as CRABBY_MAX_GAMEPADS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_world_new() -> *mut CrabbyWorld {
    crate::ponko_world_new()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_world_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_world_free(world: *mut CrabbyWorld) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_world_free(world) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_last_error() -> *const c_char {
    crate::ponko_last_error()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_last_error_kind() -> CrabbyErrorKind {
    crate::ponko_last_error_kind().into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_abi_version() -> u32 {
    crate::ponko_abi_version()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_component_register`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_component_register(
    world: *mut CrabbyWorld,
    name: *const c_char,
    size: usize,
    alignment: usize,
    out_id: *mut u32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_component_register(world, name, size, alignment, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_transform_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_transform_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_transform_component(world, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_sprite_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_sprite_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_sprite_component(world, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_spawn`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_spawn(
    world: *mut CrabbyWorld,
    values: *const CrabbyValue,
    count: usize,
    out_entity: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_spawn(world, values, count, out_entity) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_despawn`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_despawn(world: *mut CrabbyWorld, entity: u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_despawn(world, entity) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_is_alive`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_is_alive(world: *mut CrabbyWorld, entity: u64, out_alive: *mut bool) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_is_alive(world, entity, out_alive) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_world_len`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_world_len(world: *mut CrabbyWorld, out_len: *mut usize) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_world_len(world, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_get`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_get(
    world: *mut CrabbyWorld,
    entity: u64,
    id: u32,
    out_data: *mut c_void,
    size: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_get(world, entity, id, out_data, size) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_set`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_set(
    world: *mut CrabbyWorld,
    entity: u64,
    id: u32,
    data: *const c_void,
    size: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_set(world, entity, id, data, size) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_remove`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_remove(world: *mut CrabbyWorld, entity: u64, id: u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_remove(world, entity, id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_world_sort_spatial`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_world_sort_spatial(world: *mut CrabbyWorld, cell_size: f32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_world_sort_spatial(world, cell_size) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_query`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_query(
    world: *mut CrabbyWorld,
    components: *const u32,
    count: usize,
    callback: CrabbyQueryFn,
    user: *mut c_void,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_query(world, components, count, callback, user) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_query_par`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_query_par(
    world: *mut CrabbyWorld,
    components: *const u32,
    count: usize,
    callback: CrabbyQueryFn,
    user: *mut c_void,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_query_par(world, components, count, callback, user) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_text_params_default() -> CrabbyTextParams {
    crate::ponko_text_params_default()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ui_style_default() -> CrabbyUiStyle {
    crate::ponko_ui_style_default()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_sound_params_default() -> CrabbySoundParams {
    crate::ponko_sound_params_default()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_draw_rect`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_draw_rect(
    ctx: *mut CrabbyCtx,
    rect: CrabbyRect,
    color: CrabbyColor,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_draw_rect(ctx, rect, color) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_draw_line`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_draw_line(
    ctx: *mut CrabbyCtx,
    a: CrabbyVec2,
    b: CrabbyVec2,
    width: f32,
    color: CrabbyColor,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_draw_line(ctx, a, b, width, color) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_draw_circle`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_draw_circle(
    ctx: *mut CrabbyCtx,
    center: CrabbyVec2,
    radius: f32,
    color: CrabbyColor,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_draw_circle(ctx, center, radius, color) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_style`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_style(ctx: *mut CrabbyCtx, out_style: *mut CrabbyUiStyle) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_style(ctx, out_style) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_ui_style`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_ui_style(ctx: *mut CrabbyCtx, style: *const CrabbyUiStyle) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_ui_style(ctx, style) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_mouse_over_ui`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_mouse_over_ui(ctx: *mut CrabbyCtx, out_over: *mut bool) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_mouse_over_ui(ctx, out_over) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_panel`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_panel(ctx: *mut CrabbyCtx, rect: CrabbyRect) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_panel(ctx, rect) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_label`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_label(
    ctx: *mut CrabbyCtx,
    pos: CrabbyVec2,
    string: *const c_char,
    out_size: *mut CrabbyVec2,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_label(ctx, pos, string, out_size) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_button`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_button(
    ctx: *mut CrabbyCtx,
    rect: CrabbyRect,
    label: *const c_char,
    out_clicked: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_button(ctx, rect, label, out_clicked) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_load_font`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_font(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    out_font: *mut CrabbyFont,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_load_font(ctx, path, out_font) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_draw_text_ex`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_draw_text_ex(
    ctx: *mut CrabbyCtx,
    string: *const c_char,
    pos: CrabbyVec2,
    params: *const CrabbyTextParams,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_draw_text_ex(ctx, string, pos, params) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_measure_text_ex`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_measure_text_ex(
    ctx: *mut CrabbyCtx,
    string: *const c_char,
    font: CrabbyFont,
    size: f32,
    out_size: *mut CrabbyVec2,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_measure_text_ex(ctx, string, font, size, out_size) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_load_sound`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_sound(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    out_sound: *mut CrabbySound,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_load_sound(ctx, path, out_sound) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_create_sound`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_sound(
    ctx: *mut CrabbyCtx,
    sample_rate: u32,
    samples: *const f32,
    len: usize,
    out_sound: *mut CrabbySound,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_create_sound(ctx, sample_rate, samples, len, out_sound) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_play_sound`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_play_sound(
    ctx: *mut CrabbyCtx,
    sound: CrabbySound,
    params: *const CrabbySoundParams,
    out_playing: *mut CrabbyPlaying,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_play_sound(ctx, sound, params, out_playing) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_stop_sound`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_stop_sound(ctx: *mut CrabbyCtx, playing: CrabbyPlaying) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_stop_sound(ctx, playing) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_save_screenshot`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_save_screenshot(ctx: *mut CrabbyCtx, path: *const c_char) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_save_screenshot(ctx, path) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_reloaded_textures`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_reloaded_textures(
    ctx: *const CrabbyCtx,
    out_textures: *mut *const CrabbyTexture,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_reloaded_textures(ctx, out_textures, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_turns`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_turns(
    ctx: *const CrabbyCtx,
    every: u32,
    out_turns: *mut CrabbyTurns,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_turns(ctx, every, out_turns) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_net_messages_host`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_net_messages_host(
    addr: *const c_char,
    max_clients: usize,
    out_endpoint: *mut *mut CrabbyNetMessages,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_net_messages_host(addr, max_clients, out_endpoint) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_net_messages_join`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_net_messages_join(
    addr: *const c_char,
    out_endpoint: *mut *mut CrabbyNetMessages,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_net_messages_join(addr, out_endpoint) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_net_messages_loopback`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_net_messages_loopback(
    players: usize,
    out_endpoints: *mut *mut CrabbyNetMessages,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_net_messages_loopback(players, out_endpoints) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_net_messages_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_net_messages_free(endpoint: *mut CrabbyNetMessages) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_net_messages_free(endpoint) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_net_messages_set_timeout`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_net_messages_set_timeout(endpoint: *mut CrabbyNetMessages, ms: u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_net_messages_set_timeout(endpoint, ms) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_net_messages_local_port`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_net_messages_local_port(
    endpoint: *mut CrabbyNetMessages,
    out_port: *mut u16,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_net_messages_local_port(endpoint, out_port) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_net_messages_send`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_net_messages_send(
    endpoint: *mut CrabbyNetMessages,
    peer: u64,
    lane_: CrabbyLane,
    bytes: *const u8,
    len: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_net_messages_send(endpoint, peer, lane_.into(), bytes, len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_net_messages_send_to_all`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_net_messages_send_to_all(
    endpoint: *mut CrabbyNetMessages,
    lane_: CrabbyLane,
    bytes: *const u8,
    len: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_net_messages_send_to_all(endpoint, lane_.into(), bytes, len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_net_messages_poll`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_net_messages_poll(endpoint: *mut CrabbyNetMessages) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_net_messages_poll(endpoint) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_net_messages_next_event`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_net_messages_next_event(
    endpoint: *mut CrabbyNetMessages,
    out_event: *mut CrabbyMessageEvent,
    out_found: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_net_messages_next_event(endpoint, out_event.cast(), out_found) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_net_messages_disconnect`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_net_messages_disconnect(endpoint: *mut CrabbyNetMessages, peer: u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_net_messages_disconnect(endpoint, peer) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_net_messages_stats`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_net_messages_stats(
    endpoint: *mut CrabbyNetMessages,
    out_stats: *mut CrabbyMessageStats,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_net_messages_stats(endpoint, out_stats) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_net_messages_steam_host`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_net_messages_steam_host(
    app_id: u32,
    max_clients: usize,
    out_endpoint: *mut *mut CrabbyNetMessages,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_net_messages_steam_host(app_id, max_clients, out_endpoint) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_net_messages_steam_join`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_net_messages_steam_join(
    app_id: u32,
    lobby: u64,
    out_endpoint: *mut *mut CrabbyNetMessages,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_net_messages_steam_join(app_id, lobby, out_endpoint) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_net_messages_steam_peer`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_net_messages_steam_peer(
    endpoint: *mut CrabbyNetMessages,
    peer: u64,
    output: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_net_messages_steam_peer(endpoint, peer, output) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_mesh_data_add_morph_target`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_mesh_data_add_morph_target(
    data: *mut CrabbyMeshData,
    name: *const std::ffi::c_char,
    positions: *const CrabbyVec3,
    normals: *const CrabbyVec3,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_mesh_data_add_morph_target(data, name, positions, normals, count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_mesh_data_clear_morph_targets`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_mesh_data_clear_morph_targets(data: *mut CrabbyMeshData) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_mesh_data_clear_morph_targets(data) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_mesh_data_morph_target_count`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_mesh_data_morph_target_count(data: *const CrabbyMeshData) -> usize {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_mesh_data_morph_target_count(data) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_morph_target`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_morph_target(
    ctx: *mut CrabbyCtx,
    mesh: CrabbyMesh,
    name: *const std::ffi::c_char,
    out_target: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_morph_target(ctx, mesh, name, out_target) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_morph_weight`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_morph_weight(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    target: usize,
    weight: f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_morph_weight(ctx, entity_bits, target, weight) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_morph_weight`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_morph_weight(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    target: usize,
    out_weight: *mut f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_morph_weight(ctx, entity_bits, target, out_weight) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_clear_morph_weights`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_clear_morph_weights(ctx: *mut CrabbyCtx, entity_bits: u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_clear_morph_weights(ctx, entity_bits) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_mesh_data_add_morph_frame`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_mesh_data_add_morph_frame(
    data: *mut CrabbyMeshData,
    channel: usize,
    target: usize,
    weight: f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_mesh_data_add_morph_frame(data, channel, target, weight) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_clip_new`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_clip_new(
    name: *const std::ffi::c_char,
    output: *mut *mut crate::CrabbyClip,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_clip_new(name, output) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_clip_set_morph_track`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_clip_set_morph_track(
    clip: *mut crate::CrabbyClip,
    name: *const std::ffi::c_char,
    times: *const f32,
    values: *const f32,
    count: usize,
    interpolation: u32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_clip_set_morph_track(clip, name, times, values, count, interpolation) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_write_animated_morphs`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_write_animated_morphs(ctx: *mut CrabbyCtx) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_write_animated_morphs(ctx) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_nav_bake_settings_default() -> CrabbyNavBakeSettings {
    crate::ponko_nav_bake_settings_default()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_nav_sources_new() -> *mut CrabbyNavSources {
    crate::ponko_nav_sources_new()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_nav_sources_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_nav_sources_free(sources: *mut CrabbyNavSources) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_nav_sources_free(sources) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_nav_sources_add_triangles`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_nav_sources_add_triangles(
    sources_: *mut CrabbyNavSources,
    corners: *const CrabbyVec3,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_nav_sources_add_triangles(sources_, corners, count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_nav_sources_add_mesh`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_nav_sources_add_mesh(
    sources_: *mut CrabbyNavSources,
    data: *const CrabbyMeshData,
    transform: CrabbyTransform3D,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_nav_sources_add_mesh(sources_, data, transform) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_bake_navmesh`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_bake_navmesh(
    ctx: *mut CrabbyCtx,
    sources_: *const CrabbyNavSources,
    settings: *const CrabbyNavBakeSettings,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_bake_navmesh(ctx, sources_, settings) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_navmesh_sample_position`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_navmesh_sample_position(
    ctx: *mut CrabbyCtx,
    point: CrabbyVec3,
    max_distance: f32,
    out_point: *mut CrabbyVec3,
    out_found: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_navmesh_sample_position(ctx, point, max_distance, out_point, out_found) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_navmesh_find_path`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_navmesh_find_path(
    ctx: *mut CrabbyCtx,
    from: CrabbyVec3,
    to: CrabbyVec3,
    out_corners: *mut CrabbyVec3,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_navmesh_find_path(ctx, from, to, out_corners, capacity, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_nav_agent`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_nav_agent(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    speed: f32,
    radius: f32,
    acceleration: f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_nav_agent(ctx, entity_bits, speed, radius, acceleration) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_nav_destination`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_nav_destination(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    destination: *const CrabbyVec3,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_nav_destination(ctx, entity_bits, destination) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_nav_obstacle`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_nav_obstacle(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    center: CrabbyVec3,
    half_extents: CrabbyVec3,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_nav_obstacle(ctx, entity_bits, center, half_extents) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_nav_carve_obstacle`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_nav_carve_obstacle(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    center: CrabbyVec3,
    half_extents: CrabbyVec3,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_nav_carve_obstacle(ctx, entity_bits, center, half_extents) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_navmesh_update_carving`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_navmesh_update_carving(
    ctx: *mut CrabbyCtx,
    budget: usize,
    steps: *mut usize,
    pending: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_navmesh_update_carving(ctx, budget, steps, pending) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_remove_nav`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_remove_nav(ctx: *mut CrabbyCtx, entity_bits: u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_remove_nav(ctx, entity_bits) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_navmesh3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_navmesh3d(game: *mut CrabbyGame) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_navmesh3d(game) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_nav_avoidance`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_nav_avoidance(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    enabled: bool,
    weight: f32,
    neighbour_distance: f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_nav_avoidance(ctx, entity_bits, enabled, weight, neighbour_distance) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_lockstep_host`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_lockstep_host(
    game: *mut CrabbyGame,
    addr: *const c_char,
    players: u32,
    input_delay: u32,
    checksum_every: u32,
    out_port: *mut u16,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_lockstep_host(game, addr, players, input_delay, checksum_every, out_port) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_lockstep_join`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_lockstep_join(game: *mut CrabbyGame, addr: *const c_char) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_lockstep_join(game, addr) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_net_player`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_net_player(
    ctx: *mut CrabbyCtx,
    out_player: *mut u32,
    out_players: *mut u32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_net_player(ctx, out_player, out_players) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_net_state`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_net_state(ctx: *mut CrabbyCtx, out_state: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_net_state(ctx, out_state) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_input`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_input(ctx: *mut CrabbyCtx, data: *const c_void, size: usize) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_input(ctx, data, size) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_input`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_input(
    ctx: *mut CrabbyCtx,
    player: u32,
    out_data: *mut *const c_void,
    out_size: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_input(ctx, player, out_data, out_size) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_net_desync`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_net_desync(
    ctx: *mut CrabbyCtx,
    out_found: *mut bool,
    out_tick: *mut u64,
    out_player: *mut u32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_net_desync(ctx, out_found, out_tick, out_player) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_particle_settings3d_default() -> CrabbyParticleSettings3D {
    crate::ponko_particle_settings3d_default()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_particle_emitter3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_particle_emitter3d(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    settings: *const CrabbyParticleSettings3D,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_particle_emitter3d(ctx, entity_bits, settings) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_particle_burst`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_particle_burst(ctx: *mut CrabbyCtx, entity_bits: u64, count: u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_particle_burst(ctx, entity_bits, count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_particle_live_count`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_particle_live_count(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    out_count: *mut u32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_particle_live_count(ctx, entity_bits, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_remove_particle_emitter3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_remove_particle_emitter3d(ctx: *mut CrabbyCtx, entity_bits: u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_remove_particle_emitter3d(ctx, entity_bits) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_frame_stats`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_frame_stats(
    ctx: *const CrabbyCtx,
    out_stats: *mut CrabbyFrameStats,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_frame_stats(ctx, out_stats) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_gpu_uploads`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_gpu_uploads(
    ctx: *const CrabbyCtx,
    out_uploads: *mut CrabbyGpuUploads,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_gpu_uploads(ctx, out_uploads) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_perf_hud`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_perf_hud(ctx: *mut CrabbyCtx, on: bool) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_perf_hud(ctx, on) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_record_trace`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_record_trace(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    frames: u32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_record_trace(ctx, path, frames) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_last_perf_report`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_last_perf_report(
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_last_perf_report(buffer, capacity, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_profile_scope`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_profile_scope(name: *const c_char) -> *const CrabbyProfileScope {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_profile_scope(name) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_profile_begin`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_profile_begin(scope: *const CrabbyProfileScope) -> u64 {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_profile_begin(scope) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_profile_end`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_profile_end(scope: *const CrabbyProfileScope, token: u64) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_profile_end(scope, token) }
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_note_allocation(bytes: usize) {
    crate::ponko_note_allocation(bytes)
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_profiling`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_profiling(ctx: *mut CrabbyCtx, on: bool) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_profiling(ctx, on) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_perf_report`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_perf_report(
    ctx: *const CrabbyCtx,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_perf_report(ctx, buffer, capacity, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_perf_report_text`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_perf_report_text(
    report_json: *const c_char,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_perf_report_text(report_json, buffer, capacity, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_perf_check`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_perf_check(
    report_json: *const c_char,
    budget_json: *const c_char,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_perf_check(report_json, budget_json, buffer, capacity, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_perf_compare`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_perf_compare(
    report_json: *const c_char,
    baseline_json: *const c_char,
    tolerance: f32,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_perf_compare(report_json, baseline_json, tolerance, buffer, capacity, out_len) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_rigid_body(kind: u32) -> CrabbyRigidBody {
    crate::ponko_rigid_body(kind)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_collider_circle(radius: f32) -> CrabbyCollider {
    crate::ponko_collider_circle(radius)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_collider_rect(size: CrabbyVec2) -> CrabbyCollider {
    crate::ponko_collider_rect(size)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_collider_capsule(height: f32, radius: f32) -> CrabbyCollider {
    crate::ponko_collider_capsule(height, radius)
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_set_polygon_collider`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_set_polygon_collider(
    world: *mut CrabbyWorld,
    entity: u64,
    points: *const CrabbyVec2,
    count: u32,
    settings: CrabbyCollider,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_set_polygon_collider(world, entity, points, count, settings) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_collider_points`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_collider_points(
    world: *mut CrabbyWorld,
    entity: u64,
    out_points: *mut CrabbyVec2,
    out_count: *mut u32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_collider_points(world, entity, out_points, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_rigid_body_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_rigid_body_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_rigid_body_component(world, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_collider_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_collider_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_collider_component(world, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_velocity_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_velocity_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_velocity_component(world, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_physics`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_physics(game: *mut CrabbyGame, gravity: CrabbyVec2) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_physics(game, gravity) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_gravity`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_gravity(ctx: *mut crate::CrabbyCtx, gravity: CrabbyVec2) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_gravity(ctx, gravity) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_cast_ray`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_cast_ray(
    ctx: *mut crate::CrabbyCtx,
    origin: CrabbyVec2,
    direction: CrabbyVec2,
    max_distance: f32,
    out_found: *mut bool,
    out_hit: *mut CrabbyRayHit,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_cast_ray(ctx, origin, direction, max_distance, out_found, out_hit) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_physics_at`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_physics_at(
    ctx: *mut crate::CrabbyCtx,
    point: CrabbyVec2,
    out_entities: *mut CrabbyEntity,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_physics_at(ctx, point, out_entities, capacity, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_physics_in_circle`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_physics_in_circle(
    ctx: *mut crate::CrabbyCtx,
    center: CrabbyVec2,
    radius: f32,
    out_entities: *mut CrabbyEntity,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_physics_in_circle(ctx, center, radius, out_entities, capacity, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_apply_impulse`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_apply_impulse(
    ctx: *mut crate::CrabbyCtx,
    entity: u64,
    impulse: CrabbyVec2,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_apply_impulse(ctx, entity, impulse) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_body_mass`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_body_mass(
    ctx: *mut crate::CrabbyCtx,
    entity: u64,
    out_mass: *mut f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_body_mass(ctx, entity, out_mass) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_collisions`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_collisions(
    ctx: *mut crate::CrabbyCtx,
    started: bool,
    out_data: *mut *const CrabbyCollision,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_collisions(ctx, started, out_data, out_count) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_joint3d_generic(body_a: u64, body_b: u64) -> CrabbyJoint3D {
    crate::ponko_joint3d_generic(body_a, body_b).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_joint3d_fixed(body_a: u64, body_b: u64) -> CrabbyJoint3D {
    crate::ponko_joint3d_fixed(body_a, body_b).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_joint3d_spherical(body_a: u64, body_b: u64) -> CrabbyJoint3D {
    crate::ponko_joint3d_spherical(body_a, body_b).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_joint3d_revolute(body_a: u64, body_b: u64) -> CrabbyJoint3D {
    crate::ponko_joint3d_revolute(body_a, body_b).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_joint3d_prismatic(body_a: u64, body_b: u64) -> CrabbyJoint3D {
    crate::ponko_joint3d_prismatic(body_a, body_b).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_collision_groups_all() -> CrabbyCollisionGroups {
    crate::ponko_collision_groups_all()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_query_filter3d() -> CrabbyQueryFilter3D {
    crate::ponko_query_filter3d()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_rigid_body3d(kind: u32) -> CrabbyRigidBody3D {
    crate::ponko_rigid_body3d(kind)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_collider3d_sphere(radius: f32) -> CrabbyCollider3D {
    crate::ponko_collider3d_sphere(radius)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_collider3d_box(size: CrabbyVec3) -> CrabbyCollider3D {
    crate::ponko_collider3d_box(size)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_collider3d_capsule(height: f32, radius: f32) -> CrabbyCollider3D {
    crate::ponko_collider3d_capsule(height, radius)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_collider3d_mesh(mesh: CrabbyCollisionMesh) -> CrabbyCollider3D {
    crate::ponko_collider3d_mesh(mesh)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_character3d(height: f32, radius: f32) -> CrabbyCharacter3D {
    crate::ponko_character3d(height, radius)
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_rigid_body3d_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_rigid_body3d_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_rigid_body3d_component(world, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_collider3d_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_collider3d_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_collider3d_component(world, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_velocity3d_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_velocity3d_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_velocity3d_component(world, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_character3d_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_character3d_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_character3d_component(world, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_joint3d_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_joint3d_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_joint3d_component(world, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_physics3d_joint_count`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_physics3d_joint_count(ctx: *mut CrabbyCtx, out_count: *mut usize) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_physics3d_joint_count(ctx, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_physics3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_physics3d(game: *mut CrabbyGame, gravity: CrabbyVec3) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_physics3d(game, gravity) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_gravity3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_gravity3d(ctx: *mut crate::CrabbyCtx, gravity: CrabbyVec3) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_gravity3d(ctx, gravity) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_physics3d_heightfield`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_physics3d_heightfield(
    ctx: *mut crate::CrabbyCtx,
    heights: *const f32,
    count: usize,
    size: f32,
    out_mesh: *mut CrabbyCollisionMesh,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_physics3d_heightfield(ctx, heights, count, size, out_mesh) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_physics3d_convex_hull`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_physics3d_convex_hull(
    ctx: *mut crate::CrabbyCtx,
    points: *const CrabbyVec3,
    count: usize,
    out_mesh: *mut CrabbyCollisionMesh,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_physics3d_convex_hull(ctx, points, count, out_mesh) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_physics3d_trimesh`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_physics3d_trimesh(
    ctx: *mut crate::CrabbyCtx,
    vertices: *const CrabbyVec3,
    vertex_count: usize,
    indices: *const u32,
    index_count: usize,
    out_mesh: *mut CrabbyCollisionMesh,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_physics3d_trimesh(ctx, vertices, vertex_count, indices, index_count, out_mesh) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_physics3d_free_mesh`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_physics3d_free_mesh(
    ctx: *mut crate::CrabbyCtx,
    mesh: CrabbyCollisionMesh,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_physics3d_free_mesh(ctx, mesh) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_cast_ray3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_cast_ray3d(
    ctx: *mut crate::CrabbyCtx,
    origin: CrabbyVec3,
    direction: CrabbyVec3,
    max_distance: f32,
    filter: CrabbyQueryFilter3D,
    out_found: *mut bool,
    out_hit: *mut CrabbyHit3D,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_cast_ray3d(ctx, origin, direction, max_distance, filter, out_found, out_hit) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_cast_ray3d_all`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_cast_ray3d_all(
    ctx: *mut crate::CrabbyCtx,
    origin: CrabbyVec3,
    direction: CrabbyVec3,
    max_distance: f32,
    filter: CrabbyQueryFilter3D,
    out_hits: *mut CrabbyHit3D,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe {
        crate::ponko_ctx_cast_ray3d_all(
            ctx,
            origin,
            direction,
            max_distance,
            filter,
            out_hits,
            capacity,
            out_count,
        )
    })
    .into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_cast_sphere3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_cast_sphere3d(
    ctx: *mut crate::CrabbyCtx,
    origin: CrabbyVec3,
    radius: f32,
    direction: CrabbyVec3,
    max_distance: f32,
    filter: CrabbyQueryFilter3D,
    out_found: *mut bool,
    out_hit: *mut CrabbyHit3D,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe {
        crate::ponko_ctx_cast_sphere3d(ctx, origin, radius, direction, max_distance, filter, out_found, out_hit)
    })
    .into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_cast_capsule3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_cast_capsule3d(
    ctx: *mut crate::CrabbyCtx,
    origin: CrabbyVec3,
    height: f32,
    radius: f32,
    direction: CrabbyVec3,
    max_distance: f32,
    filter: CrabbyQueryFilter3D,
    out_found: *mut bool,
    out_hit: *mut CrabbyHit3D,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe {
        crate::ponko_ctx_cast_capsule3d(
            ctx,
            origin,
            height,
            radius,
            direction,
            max_distance,
            filter,
            out_found,
            out_hit,
        )
    })
    .into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_overlap_sphere3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_overlap_sphere3d(
    ctx: *mut crate::CrabbyCtx,
    center: CrabbyVec3,
    radius: f32,
    filter: CrabbyQueryFilter3D,
    out_entities: *mut CrabbyEntity,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_overlap_sphere3d(ctx, center, radius, filter, out_entities, capacity, out_count) })
        .into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_overlap_box3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_overlap_box3d(
    ctx: *mut crate::CrabbyCtx,
    center: CrabbyVec3,
    size: CrabbyVec3,
    rotation: CrabbyQuat,
    filter: CrabbyQueryFilter3D,
    out_entities: *mut CrabbyEntity,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_overlap_box3d(ctx, center, size, rotation, filter, out_entities, capacity, out_count) })
        .into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_overlap_capsule3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_overlap_capsule3d(
    ctx: *mut crate::CrabbyCtx,
    center: CrabbyVec3,
    height: f32,
    radius: f32,
    filter: CrabbyQueryFilter3D,
    out_entities: *mut CrabbyEntity,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe {
        crate::ponko_ctx_overlap_capsule3d(ctx, center, height, radius, filter, out_entities, capacity, out_count)
    })
    .into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_apply_impulse3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_apply_impulse3d(
    ctx: *mut crate::CrabbyCtx,
    entity: u64,
    impulse: CrabbyVec3,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_apply_impulse3d(ctx, entity, impulse) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_body_mass3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_body_mass3d(
    ctx: *mut crate::CrabbyCtx,
    entity: u64,
    out_mass: *mut f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_body_mass3d(ctx, entity, out_mass) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_body_center_of_mass3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_body_center_of_mass3d(
    ctx: *mut crate::CrabbyCtx,
    entity: u64,
    out_center: *mut CrabbyVec3,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_body_center_of_mass3d(ctx, entity, out_center) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_terrain_collider(mesh: CrabbyCollisionMesh) -> CrabbyTerrainCollider {
    crate::ponko_terrain_collider(mesh)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_collider_focus(radius: f32, keep_radius: f32) -> CrabbyColliderFocus {
    crate::ponko_collider_focus(radius, keep_radius)
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_terrain_collider_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_terrain_collider_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_terrain_collider_component(world, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_collider_focus_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_collider_focus_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_collider_focus_component(world, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_collision_mesh_data_heightfield`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_collision_mesh_data_heightfield(
    heights: *const f32,
    count: usize,
    size: f32,
) -> *mut CrabbyCollisionMeshData {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_collision_mesh_data_heightfield(heights, count, size) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_collision_mesh_data_trimesh`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_collision_mesh_data_trimesh(
    vertices: *const CrabbyVec3,
    vertex_count: usize,
    indices: *const u32,
    index_count: usize,
) -> *mut CrabbyCollisionMeshData {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_collision_mesh_data_trimesh(vertices, vertex_count, indices, index_count) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_collision_mesh_data_convex_hull`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_collision_mesh_data_convex_hull(
    points: *const CrabbyVec3,
    count: usize,
) -> *mut CrabbyCollisionMeshData {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_collision_mesh_data_convex_hull(points, count) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_collision_mesh_data_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_collision_mesh_data_free(data: *mut CrabbyCollisionMeshData) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_collision_mesh_data_free(data) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_physics3d_add_mesh`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_physics3d_add_mesh(
    ctx: *mut CrabbyCtx,
    data: *const CrabbyCollisionMeshData,
    out_mesh: *mut CrabbyCollisionMesh,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_physics3d_add_mesh(ctx, data, out_mesh) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_physics3d_stream_colliders`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_physics3d_stream_colliders(
    ctx: *mut CrabbyCtx,
    enabled: bool,
    max_new: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_physics3d_stream_colliders(ctx, enabled, max_new) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_physics3d_is_collider_active`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_physics3d_is_collider_active(
    ctx: *mut CrabbyCtx,
    entity: u64,
    out_active: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_physics3d_is_collider_active(ctx, entity, out_active) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_QUERY_CACHE_ALWAYS as CRABBY_QUERY_CACHE_ALWAYS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_QUERY_CACHE_AUTO as CRABBY_QUERY_CACHE_AUTO;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_QUERY_CACHE_NEVER as CRABBY_QUERY_CACHE_NEVER;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_query_cache_policy_default() -> CrabbyQueryCachePolicy {
    crate::ponko_query_cache_policy_default()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_query_cache_policy_off() -> CrabbyQueryCachePolicy {
    crate::ponko_query_cache_policy_off()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_world_set_query_cache_policy`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_world_set_query_cache_policy(
    world: *mut CrabbyWorld,
    policy: *const CrabbyQueryCachePolicy,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_world_set_query_cache_policy(world, policy) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_world_query_cache_policy`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_world_query_cache_policy(
    world: *mut CrabbyWorld,
    out_policy: *mut CrabbyQueryCachePolicy,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_world_query_cache_policy(world, out_policy) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_world_query_cache_stats`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_world_query_cache_stats(
    world: *mut CrabbyWorld,
    out_stats: *mut CrabbyQueryCacheStats,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_world_query_cache_stats(world, out_stats) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_query_cached`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_query_cached(
    world: *mut CrabbyWorld,
    components: *const u32,
    count: usize,
    filter: *const CrabbyFilter,
    options: *const CrabbyQueryOptions,
    cache: u32,
    callback: CrabbyQueryFn,
    user: *mut c_void,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_query_cached(world, components, count, filter, options, cache, callback, user) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ragdoll_bone(joint: *const c_char, radius: f32) -> CrabbyRagdollBone {
    crate::ponko_ragdoll_bone(joint, radius)
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_start_ragdoll`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_start_ragdoll(
    ctx: *mut CrabbyCtx,
    entity: u64,
    bones: *const CrabbyRagdollBone,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_start_ragdoll(ctx, entity, bones, count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_stop_ragdoll`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_stop_ragdoll(ctx: *mut CrabbyCtx, entity: u64, seconds: f32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_stop_ragdoll(ctx, entity, seconds) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ragdoll_body`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ragdoll_body(
    ctx: *mut CrabbyCtx,
    entity: u64,
    joint: usize,
    out_body: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ragdoll_body(ctx, entity, joint, out_body) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ragdoll_state`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ragdoll_state(
    ctx: *mut CrabbyCtx,
    entity: u64,
    out_ragdolled: *mut bool,
    out_blending: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ragdoll_state(ctx, entity, out_ragdolled, out_blending) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_component_reflect`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_component_reflect(
    world: *mut CrabbyWorld,
    component_id: u32,
    fields: *const CrabbyField,
    len: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_component_reflect(world, component_id, fields, len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_world_save`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_world_save(
    world: *mut CrabbyWorld,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_world_save(world, buffer, capacity, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_world_load`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_world_load(world: *mut CrabbyWorld, json: *const c_char) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_world_load(world, json) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_world_describe_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_world_describe_component(
    world: *mut CrabbyWorld,
    name: *const c_char,
    schema: *const c_char,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_world_describe_component(world, name, schema) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_world_describe_resource`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_world_describe_resource(
    world: *mut CrabbyWorld,
    name: *const c_char,
    schema: *const c_char,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_world_describe_resource(world, name, schema) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_capabilities_json`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_capabilities_json(
    ctx: *const crate::CrabbyCtx,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_capabilities_json(ctx, buffer, capacity, out_len) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_seq_newer(a: u16, b: u16) -> bool {
    crate::ponko_seq_newer(a, b)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_seq_diff(a: u16, b: u16) -> i16 {
    crate::ponko_seq_diff(a, b)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_stamp_newer(a: CrabbyUpdateStamp, b: CrabbyUpdateStamp) -> bool {
    crate::ponko_stamp_newer(a, b)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_authority_new(owner: u64) -> CrabbyAuthority {
    crate::ponko_authority_new(owner)
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_authority_handoff`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_authority_handoff(
    authority: *mut CrabbyAuthority,
    new_owner: u64,
    out_epoch: *mut u16,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_authority_handoff(authority, new_owner, out_epoch) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_authority_adopt`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_authority_adopt(
    authority: *mut CrabbyAuthority,
    owner: u64,
    epoch: u16,
    out_changed: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_authority_adopt(authority, owner, epoch, out_changed) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_authority_check`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_authority_check(
    authority: *const CrabbyAuthority,
    from: u64,
    epoch: u16,
    out_check: *mut CrabbyAuthorityCheck,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_authority_check(authority, from, epoch, out_check.cast()) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_latest_wins_new() -> *mut CrabbyLatestWins {
    crate::ponko_latest_wins_new()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_latest_wins_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_latest_wins_free(filter: *mut CrabbyLatestWins) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_latest_wins_free(filter) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_latest_wins_accept`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_latest_wins_accept(
    f: *mut CrabbyLatestWins,
    key: u64,
    stamp: CrabbyUpdateStamp,
    out_accepted: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_latest_wins_accept(f, key, stamp, out_accepted) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_latest_wins_forget`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_latest_wins_forget(f: *mut CrabbyLatestWins, key: u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_latest_wins_forget(f, key) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_latest_wins_clear`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_latest_wins_clear(f: *mut CrabbyLatestWins) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_latest_wins_clear(f) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_latest_wins_len`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_latest_wins_len(f: *const CrabbyLatestWins) -> usize {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_latest_wins_len(f) }
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_snapshot_config_default() -> CrabbySnapshotConfig {
    crate::ponko_snapshot_config_default()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_snapshot_stream_new`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_snapshot_stream_new(
    config: *const CrabbySnapshotConfig,
    out_stream: *mut *mut CrabbySnapshotStream,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_snapshot_stream_new(config, out_stream) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_snapshot_stream_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_snapshot_stream_free(s: *mut CrabbySnapshotStream) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_snapshot_stream_free(s) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_snapshot_stream_push`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_snapshot_stream_push(
    s: *mut CrabbySnapshotStream,
    remote_time: f64,
    local_now: f64,
    pose: *const CrabbyPose,
    out_result: *mut CrabbySnapshotPush,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_snapshot_stream_push(s, remote_time, local_now, pose, out_result.cast()) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_snapshot_stream_sample`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_snapshot_stream_sample(
    s: *const CrabbySnapshotStream,
    now: f64,
    out_pose: *mut CrabbyPose,
    out_state: *mut CrabbySnapshotState,
    out_found: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_snapshot_stream_sample(s, now, out_pose, out_state.cast(), out_found) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_snapshot_stream_set_delay`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_snapshot_stream_set_delay(s: *mut CrabbySnapshotStream, delay: f64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_snapshot_stream_set_delay(s, delay) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_snapshot_stream_clear`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_snapshot_stream_clear(s: *mut CrabbySnapshotStream) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_snapshot_stream_clear(s) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_snapshot_stream_reset`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_snapshot_stream_reset(s: *mut CrabbySnapshotStream) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_snapshot_stream_reset(s) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_snapshot_stream_len`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_snapshot_stream_len(s: *const CrabbySnapshotStream) -> usize {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_snapshot_stream_len(s) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_snapshot_stream_clock_offset`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_snapshot_stream_clock_offset(
    s: *const CrabbySnapshotStream,
    out_offset: *mut f64,
    out_known: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_snapshot_stream_clock_offset(s, out_offset, out_known) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_interest_config_default() -> CrabbyInterestConfig {
    crate::ponko_interest_config_default()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_interest_new`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_interest_new(
    config: *const CrabbyInterestConfig,
    out_interest: *mut *mut CrabbyInterest,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_interest_new(config, out_interest) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_interest_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_interest_free(i: *mut CrabbyInterest) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_interest_free(i) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_interest_update`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_interest_update(
    i: *mut CrabbyInterest,
    tick: u64,
    viewers: *const CrabbyInterestViewer,
    viewer_count: usize,
    entities: *const CrabbyInterestEntity,
    entity_count: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_interest_update(i, tick, viewers, viewer_count, entities, entity_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_interest_entries`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_interest_entries(
    i: *const CrabbyInterest,
    peer: u64,
    out_entries: *mut CrabbyInterestEntry,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_interest_entries(i, peer, out_entries, capacity, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_interest_changes`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_interest_changes(
    i: *const CrabbyInterest,
    out_changes: *mut CrabbyInterestChange,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_interest_changes(i, out_changes.cast(), capacity, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_interest_remove_peer`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_interest_remove_peer(i: *mut CrabbyInterest, peer: u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_interest_remove_peer(i, peer) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_residency_budget`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_residency_budget(
    ctx: *mut CrabbyCtx,
    kind: u32,
    has_budget: bool,
    bytes: u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_residency_budget(ctx, kind, has_budget, bytes) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_residency`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_residency(
    ctx: *mut CrabbyCtx,
    kind: u32,
    out_stats: *mut CrabbyResidencyStats,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_residency(ctx, kind, out_stats) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_is_resident`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_is_resident(
    ctx: *mut CrabbyCtx,
    kind: u32,
    name: *const c_char,
    out_resident: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_is_resident(ctx, kind, name, out_resident) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_asset_state`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_asset_state(
    ctx: *mut CrabbyCtx,
    kind: u32,
    name: *const c_char,
    out_state: *mut u32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_asset_state(ctx, kind, name, out_state) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_asset_error`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_asset_error(
    ctx: *mut CrabbyCtx,
    kind: u32,
    name: *const c_char,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_asset_error(ctx, kind, name, buffer, capacity, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_resident_texture`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_resident_texture(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    pin: bool,
    out_texture: *mut CrabbyTexture,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_resident_texture(ctx, path, pin, out_texture) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_resident_mesh`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_resident_mesh(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    pin: bool,
    out_mesh: *mut CrabbyMesh,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_resident_mesh(ctx, path, pin, out_mesh) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_unpin_resident`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_unpin_resident(
    ctx: *mut CrabbyCtx,
    kind: u32,
    path: *const c_char,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_unpin_resident(ctx, kind, path) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_stream_texture`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_stream_texture(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    out_texture: *mut CrabbyTexture,
    out_ready: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_stream_texture(ctx, path, out_texture, out_ready) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_stream_mesh`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_stream_mesh(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    out_mesh: *mut CrabbyMesh,
    out_ready: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_stream_mesh(ctx, path, out_mesh, out_ready) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_upload_budget`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_upload_budget(
    ctx: *mut CrabbyCtx,
    has_budget: bool,
    bytes: u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_upload_budget(ctx, has_budget, bytes) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_deterministic_uploads`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_deterministic_uploads(ctx: *mut CrabbyCtx, on: bool) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_deterministic_uploads(ctx, on) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_upload_stats`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_upload_stats(
    ctx: *mut CrabbyCtx,
    out_stats: *mut CrabbyUploadStats,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_upload_stats(ctx, out_stats) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_finish_uploads`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_finish_uploads(ctx: *mut CrabbyCtx) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_finish_uploads(ctx) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_streamed_lod`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_streamed_lod(
    ctx: *mut CrabbyCtx,
    entity: u64,
    files: *const *const c_char,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_streamed_lod(ctx, entity, files, count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_resource_set`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_resource_set(
    world: *mut CrabbyWorld,
    name: *const c_char,
    data: *const c_void,
    size: usize,
    alignment: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_resource_set(world, name, data, size, alignment) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_resource_get`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_resource_get(
    world: *mut CrabbyWorld,
    name: *const c_char,
    out: *mut c_void,
    size: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_resource_get(world, name, out, size) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_resource_remove`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_resource_remove(world: *mut CrabbyWorld, name: *const c_char) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_resource_remove(world, name) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_resource_reflect`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_resource_reflect(
    world: *mut CrabbyWorld,
    name: *const c_char,
    size: usize,
    alignment: usize,
    fields: *const CrabbyField,
    len: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_resource_reflect(world, name, size, alignment, fields, len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_load_scene`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_scene(ctx: *mut CrabbyCtx, path: *const c_char) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_load_scene(ctx, path) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_load_scene_lockstep`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_scene_lockstep(ctx: *mut CrabbyCtx, path: *const c_char) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_load_scene_lockstep(ctx, path) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_save_scene`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_save_scene(ctx: *mut CrabbyCtx, path: *const c_char) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_save_scene(ctx, path) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_spawn_prefab`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_spawn_prefab(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    at: CrabbyVec2,
    out_entity: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_spawn_prefab(ctx, path, at, out_entity) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_save_prefab`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_save_prefab(ctx: *mut CrabbyCtx, entity: u64, path: *const c_char) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_save_prefab(ctx, entity, path) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_SHADER_COMPILED as CRABBY_SHADER_COMPILED;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_SHADER_COMPUTE as CRABBY_SHADER_COMPUTE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_SHADER_EFFECT as CRABBY_SHADER_EFFECT;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_SHADER_FAILED as CRABBY_SHADER_FAILED;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_SHADER_MATERIAL as CRABBY_SHADER_MATERIAL;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_SHADER_RELOAD_FAILED as CRABBY_SHADER_RELOAD_FAILED;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_SHADER_RELOADED as CRABBY_SHADER_RELOADED;
/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_shader_status_count`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_shader_status_count(ctx: *const CrabbyCtx, out_count: *mut usize) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_shader_status_count(ctx, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_shader_status`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_shader_status(
    ctx: *const CrabbyCtx,
    index: usize,
    out_status: *mut CrabbyShaderStatus,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_shader_status(ctx, index, out_status) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_shader_status_name`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_shader_status_name(
    ctx: *const CrabbyCtx,
    index: usize,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_shader_status_name(ctx, index, buffer, capacity, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_shader_status_text`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_shader_status_text(
    ctx: *const CrabbyCtx,
    index: usize,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_shader_status_text(ctx, index, buffer, capacity, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_shader_status_json`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_shader_status_json(
    ctx: *const CrabbyCtx,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_shader_status_json(ctx, buffer, capacity, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_mesh_data_set_skin`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_mesh_data_set_skin(
    data: *mut CrabbyMeshData,
    joints: *const u16,
    weights: *const f32,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_mesh_data_set_skin(data, joints, weights, count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_skeleton`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_skeleton(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    joints: *const CrabbyJoint,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_skeleton(ctx, entity_bits, joints, count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_load_skeleton`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_skeleton(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    path: *const c_char,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_load_skeleton(ctx, entity_bits, path) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_create_skinned_mesh`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_skinned_mesh(
    ctx: *mut CrabbyCtx,
    data: *const CrabbyMeshData,
    material: CrabbyMaterial,
    skeleton_entity: u64,
    out_mesh: *mut CrabbyMesh,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_create_skinned_mesh(ctx, data, material, skeleton_entity, out_mesh) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_joint_count`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_joint_count(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_joint_count(ctx, entity_bits, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_find_joint`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_find_joint(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    name: *const c_char,
    out_found: *mut bool,
    out_joint: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_find_joint(ctx, entity_bits, name, out_found, out_joint) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_joint_pose`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_joint_pose(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    joint: usize,
    out_pose: *mut CrabbyTransform3D,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_joint_pose(ctx, entity_bits, joint, out_pose) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_joint_pose`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_joint_pose(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    joint: usize,
    pose: *const CrabbyTransform3D,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_joint_pose(ctx, entity_bits, joint, pose) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_reset_pose`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_reset_pose(ctx: *mut CrabbyCtx, entity_bits: u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_reset_pose(ctx, entity_bits) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_joint_matrix`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_joint_matrix(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    joint: usize,
    out_matrix: *mut f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_joint_matrix(ctx, entity_bits, joint, out_matrix) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_skin_of`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_skin_of(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    skeleton_entity: u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_skin_of(ctx, entity_bits, skeleton_entity) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_attach_to_joint`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_attach_to_joint(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    skeleton_entity: u64,
    joint: usize,
    offset: *const CrabbyTransform3D,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_attach_to_joint(ctx, entity_bits, skeleton_entity, joint, offset) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_load_clip`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_clip(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    name: *const c_char,
    out_clip: *mut *mut CrabbyClip,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_load_clip(ctx, path, name, out_clip) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_clip_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_clip_free(clip: *mut CrabbyClip) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_clip_free(clip) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_clip_duration`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_clip_duration(clip: *const CrabbyClip) -> f32 {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_clip_duration(clip) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_sample_clip`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_sample_clip(
    ctx: *mut CrabbyCtx,
    clip: *const CrabbyClip,
    entity_bits: u64,
    time: f32,
    weight: f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_sample_clip(ctx, clip, entity_bits, time, weight) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_two_bone_ik`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_two_bone_ik(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    upper: usize,
    middle: usize,
    end: usize,
    target: CrabbyVec3,
    pole: CrabbyVec3,
    weight: f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_two_bone_ik(ctx, entity_bits, upper, middle, end, target, pole, weight) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_look_at_joint`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_look_at_joint(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    joint: usize,
    axis: CrabbyVec3,
    target: CrabbyVec3,
    max_angle: f32,
    weight: f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_look_at_joint(ctx, entity_bits, joint, axis, target, max_angle, weight) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_draw_skeleton`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_draw_skeleton(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    color: CrabbyColor,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_draw_skeleton(ctx, entity_bits, color) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_lockstep_steam_host`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_lockstep_steam_host(
    game: *mut CrabbyGame,
    app_id: u32,
    players: u32,
    input_delay: u32,
    checksum_every: u32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_lockstep_steam_host(game, app_id, players, input_delay, checksum_every) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_game_lockstep_steam_join`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_game_lockstep_steam_join(
    game: *mut CrabbyGame,
    app_id: u32,
    lobby: u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_game_lockstep_steam_join(game, app_id, lobby) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_steam_lobby_from_args`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_steam_lobby_from_args(out_lobby: *mut u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_steam_lobby_from_args(out_lobby) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_steam_wait_for_invite`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_steam_wait_for_invite(
    app_id: u32,
    timeout_ms: u32,
    out_lobby: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_steam_wait_for_invite(app_id, timeout_ms, out_lobby) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_steam_lobby`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_steam_lobby(ctx: *mut CrabbyCtx, out_lobby: *mut u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_steam_lobby(ctx, out_lobby) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_steam_invite`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_steam_invite(ctx: *mut CrabbyCtx) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_steam_invite(ctx) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_steam_lobby_create`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_steam_lobby_create(
    app_id: u32,
    visibility: u32,
    max_members: u32,
    output: *mut *mut CrabbySteamLobby,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_steam_lobby_create(app_id, visibility, max_members, output) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_steam_lobby_join`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_steam_lobby_join(
    app_id: u32,
    id: u64,
    output: *mut *mut CrabbySteamLobby,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_steam_lobby_join(app_id, id, output) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_steam_lobby_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_steam_lobby_free(lobby: *mut CrabbySteamLobby) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_steam_lobby_free(lobby) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_steam_lobby_poll`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_steam_lobby_poll(lobby: *mut CrabbySteamLobby) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_steam_lobby_poll(lobby) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_steam_lobby_leave`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_steam_lobby_leave(lobby: *mut CrabbySteamLobby) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_steam_lobby_leave(lobby) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_steam_lobby_id`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_steam_lobby_id(lobby: *mut CrabbySteamLobby, output: *mut u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_steam_lobby_id(lobby, output) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_steam_lobby_invite`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_steam_lobby_invite(lobby: *mut CrabbySteamLobby) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_steam_lobby_invite(lobby) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_steam_lobby_set_metadata`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_steam_lobby_set_metadata(
    lobby: *mut CrabbySteamLobby,
    key: *const c_char,
    value: *const c_char,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_steam_lobby_set_metadata(lobby, key, value) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_steam_lobby_metadata`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_steam_lobby_metadata(
    lobby: *mut CrabbySteamLobby,
    key: *const c_char,
    buffer: *mut c_char,
    capacity: usize,
    needed: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_steam_lobby_metadata(lobby, key, buffer, capacity, needed) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_steam_lobby_members`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_steam_lobby_members(
    lobby: *mut CrabbySteamLobby,
    members: *mut u64,
    capacity: usize,
    needed: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_steam_lobby_members(lobby, members, capacity, needed) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_net_messages_steam_lobby_id`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_net_messages_steam_lobby_id(
    endpoint: *mut crate::CrabbyNetMessages,
    output: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_net_messages_steam_lobby_id(endpoint, output) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_net_messages_steam_lobby_invite`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_net_messages_steam_lobby_invite(
    endpoint: *mut crate::CrabbyNetMessages,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_net_messages_steam_lobby_invite(endpoint) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_net_messages_steam_lobby_set_metadata`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_net_messages_steam_lobby_set_metadata(
    endpoint: *mut crate::CrabbyNetMessages,
    key: *const c_char,
    value: *const c_char,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_net_messages_steam_lobby_set_metadata(endpoint, key, value) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_net_messages_steam_lobby_metadata`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_net_messages_steam_lobby_metadata(
    endpoint: *mut crate::CrabbyNetMessages,
    key: *const c_char,
    buffer: *mut c_char,
    capacity: usize,
    needed: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_net_messages_steam_lobby_metadata(endpoint, key, buffer, capacity, needed) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_net_messages_steam_lobby_members`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_net_messages_steam_lobby_members(
    endpoint: *mut crate::CrabbyNetMessages,
    members: *mut u64,
    capacity: usize,
    needed: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_net_messages_steam_lobby_members(endpoint, members, capacity, needed) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_cells_new(cell_size: f32, load_radius: f32, keep_radius: f32) -> *mut CrabbyCellStreamer {
    crate::ponko_cells_new(cell_size, load_radius, keep_radius)
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_cells_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_cells_free(cells: *mut CrabbyCellStreamer) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_cells_free(cells) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_cells_update`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_cells_update(
    cells: *mut CrabbyCellStreamer,
    at: CrabbyVec3,
    max_new: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_cells_update(cells, at, max_new) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_cells_to_load`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_cells_to_load(
    cells: *mut CrabbyCellStreamer,
    out_cells: *mut *const CrabbyCell,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_cells_to_load(cells, out_cells, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_cells_to_drop`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_cells_to_drop(
    cells: *mut CrabbyCellStreamer,
    out_cells: *mut *const CrabbyCell,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_cells_to_drop(cells, out_cells, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_cells_is_loaded`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_cells_is_loaded(
    cells: *mut CrabbyCellStreamer,
    cell: CrabbyCell,
    out_loaded: *mut bool,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_cells_is_loaded(cells, cell, out_loaded, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_cells_clear`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_cells_clear(cells: *mut CrabbyCellStreamer) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_cells_clear(cells) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_terrain_tile_data_new`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_terrain_tile_data_new(
    heights: *const f32,
    count: usize,
    size: f32,
) -> *mut CrabbyTerrainTileData {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_terrain_tile_data_new(heights, count, size) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_terrain_tile_data_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_terrain_tile_data_free(data: *mut CrabbyTerrainTileData) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_terrain_tile_data_free(data) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_upload_terrain_tile`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_upload_terrain_tile(
    ctx: *mut CrabbyCtx,
    data: *const CrabbyTerrainTileData,
    material: CrabbyMaterial,
    out_tile: *mut CrabbyTerrainTile,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_upload_terrain_tile(ctx, data, material, out_tile) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_free_mesh`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_free_mesh(ctx: *mut CrabbyCtx, mesh: CrabbyMesh) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_free_mesh(ctx, mesh) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_is_mesh_live`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_is_mesh_live(
    ctx: *mut CrabbyCtx,
    mesh: CrabbyMesh,
    out_live: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_is_mesh_live(ctx, mesh, out_live) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_free_terrain_tile`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_free_terrain_tile(ctx: *mut CrabbyCtx, tile: CrabbyTerrainTile) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_free_terrain_tile(ctx, tile) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_far_terrain_new`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_far_terrain_new(
    heights: *const f32,
    count: usize,
    columns: usize,
    corner: CrabbyVec2,
    spacing: f32,
    cell_size: f32,
    material: CrabbyMaterial,
) -> *mut CrabbyFarTerrain {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_far_terrain_new(heights, count, columns, corner, spacing, cell_size, material) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_far_terrain_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_far_terrain_free(far: *mut CrabbyFarTerrain) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_far_terrain_free(far) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_far_terrain_cut`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_far_terrain_cut(
    far: *mut CrabbyFarTerrain,
    cell: CrabbyCell,
    cut: bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_far_terrain_cut(far, cell, cut) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_far_terrain_is_cut`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_far_terrain_is_cut(
    far: *mut CrabbyFarTerrain,
    cell: CrabbyCell,
    out_cut: *mut bool,
    out_meshes: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_far_terrain_is_cut(far, cell, out_cut, out_meshes) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_far_terrain_update`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_far_terrain_update(far: *mut CrabbyFarTerrain, ctx: *mut CrabbyCtx) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_far_terrain_update(far, ctx) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_far_terrain_clear`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_far_terrain_clear(far: *mut CrabbyFarTerrain, ctx: *mut CrabbyCtx) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_far_terrain_clear(far, ctx) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_trail3d_new(lifetime: f32, width: f32) -> CrabbyTrail3D {
    crate::ponko_trail3d_new(lifetime, width)
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_trail3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_trail3d(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    trail: *const CrabbyTrail3D,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_trail3d(ctx, entity_bits, trail) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_trail3d_point_count`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_trail3d_point_count(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    out_count: *mut u32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_trail3d_point_count(ctx, entity_bits, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_clear_trail3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_clear_trail3d(ctx: *mut CrabbyCtx, entity_bits: u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_clear_trail3d(ctx, entity_bits) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_trail3d_emitting`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_trail3d_emitting(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    emitting: bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_trail3d_emitting(ctx, entity_bits, emitting) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_remove_trail3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_remove_trail3d(ctx: *mut CrabbyCtx, entity_bits: u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_remove_trail3d(ctx, entity_bits) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_line3d_new(width: f32, color: CrabbyColor) -> CrabbyLine3D {
    crate::ponko_line3d_new(width, color)
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_line3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_line3d(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    line: *const CrabbyLine3D,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_line3d(ctx, entity_bits, line) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_remove_line3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_remove_line3d(ctx: *mut CrabbyCtx, entity_bits: u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_remove_line3d(ctx, entity_bits) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_mesh_data_decal`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_mesh_data_decal(
    data: *const CrabbyMeshData,
    projector: *const f32,
    lift: f32,
) -> *mut CrabbyMeshData {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_mesh_data_decal(data, projector, lift) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_load_mesh_data`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_mesh_data(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    out_data: *mut *mut CrabbyMeshData,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_load_mesh_data(ctx, path, out_data) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_text3d_default() -> CrabbyText3D {
    crate::ponko_text3d_default()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_text3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_text3d(
    ctx: *mut CrabbyCtx,
    entity: u64,
    string: *const c_char,
    value: *const CrabbyText3D,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_text3d(ctx, entity, string, value) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_text3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_text3d(
    ctx: *mut CrabbyCtx,
    entity: u64,
    output: *mut CrabbyText3D,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_text3d(ctx, entity, output) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_text3d_text`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_text3d_text(
    ctx: *mut CrabbyCtx,
    entity: u64,
    buffer: *mut c_char,
    capacity: usize,
    length: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_text3d_text(ctx, entity, buffer, capacity, length) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_remove_text3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_remove_text3d(ctx: *mut CrabbyCtx, entity: u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_remove_text3d(ctx, entity) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_DECAL_NO_IMAGE as CRABBY_DECAL_NO_IMAGE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_MESH_NONE as CRABBY_MESH_NONE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_TERRAIN_LAYERS as CRABBY_TERRAIN_LAYERS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_TERRAIN_LODS as CRABBY_TERRAIN_LODS;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_TEXTURE_COLOR as CRABBY_TEXTURE_COLOR;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_TEXTURE_DATA as CRABBY_TEXTURE_DATA;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_sun_direction_at(hour: f32) -> CrabbyVec3 {
    crate::ponko_sun_direction_at(hour)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_quat_identity() -> CrabbyQuat {
    crate::ponko_quat_identity()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_quat_from_axis_angle(axis: CrabbyVec3, angle: f32) -> CrabbyQuat {
    crate::ponko_quat_from_axis_angle(axis, angle)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_quat_mul(a: CrabbyQuat, b: CrabbyQuat) -> CrabbyQuat {
    crate::ponko_quat_mul(a, b)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_quat_rotate(q: CrabbyQuat, v: CrabbyVec3) -> CrabbyVec3 {
    crate::ponko_quat_rotate(q, v)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_transform3d_at(position: CrabbyVec3) -> CrabbyTransform3D {
    crate::ponko_transform3d_at(position)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_transform3d_looking_at(t: CrabbyTransform3D, target: CrabbyVec3) -> CrabbyTransform3D {
    crate::ponko_transform3d_looking_at(t, target)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_transform3d_forward(t: CrabbyTransform3D) -> CrabbyVec3 {
    crate::ponko_transform3d_forward(t)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_model(mesh: CrabbyMesh) -> CrabbyModel {
    crate::ponko_model(mesh)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_material_desc(color: CrabbyColor) -> CrabbyMaterialDesc {
    crate::ponko_material_desc(color)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_terrain_material_desc(
    layers: CrabbyTextureArray,
    weights: CrabbyTextureArray,
    corner: CrabbyVec2,
    size: CrabbyVec2,
) -> CrabbyTerrainMaterialDesc {
    crate::ponko_terrain_material_desc(layers, weights, corner, size)
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_create_texture_array`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_texture_array(
    ctx: *mut CrabbyCtx,
    width: u32,
    height: u32,
    rgba: *const u8,
    len: usize,
    kind: u32,
    out_array: *mut CrabbyTextureArray,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_create_texture_array(ctx, width, height, rgba, len, kind, out_array) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_load_texture_array`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_texture_array(
    ctx: *mut CrabbyCtx,
    paths: *const *const c_char,
    count: usize,
    kind: u32,
    out_array: *mut CrabbyTextureArray,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_load_texture_array(ctx, paths, count, kind, out_array) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_create_terrain_material`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_terrain_material(
    ctx: *mut CrabbyCtx,
    desc: *const CrabbyTerrainMaterialDesc,
    out_material: *mut CrabbyMaterial,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_create_terrain_material(ctx, desc, out_material) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_transform3d_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_transform3d_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_transform3d_component(world, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_model_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_model_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_model_component(world, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_terrain_tile_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_terrain_tile_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_terrain_tile_component(world, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_mesh_data_new`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_mesh_data_new(
    positions: *const CrabbyVec3,
    normals: *const CrabbyVec3,
    uvs: *const CrabbyVec2,
    vertex_count: usize,
    indices: *const u32,
    index_count: usize,
) -> *mut CrabbyMeshData {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_mesh_data_new(positions, normals, uvs, vertex_count, indices, index_count) }
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_mesh_data_cube(size: f32) -> *mut CrabbyMeshData {
    crate::ponko_mesh_data_cube(size)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_mesh_data_plane(size: f32) -> *mut CrabbyMeshData {
    crate::ponko_mesh_data_plane(size)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_mesh_data_sphere(radius: f32, segments: u32) -> *mut CrabbyMeshData {
    crate::ponko_mesh_data_sphere(radius, segments)
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_mesh_data_heightfield`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_mesh_data_heightfield(
    heights: *const f32,
    count: usize,
    size: f32,
    step: u32,
) -> *mut CrabbyMeshData {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_mesh_data_heightfield(heights, count, size, step) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_mesh_data_set_colors`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_mesh_data_set_colors(
    data: *mut CrabbyMeshData,
    colors: *const CrabbyColor,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_mesh_data_set_colors(data, colors, count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_mesh_data_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_mesh_data_free(data: *mut CrabbyMeshData) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_mesh_data_free(data) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_mesh_data_view`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_mesh_data_view(
    data: *const CrabbyMeshData,
    out_view: *mut CrabbyMeshView,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_mesh_data_view(data, out_view) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_create_material`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_material(
    ctx: *mut CrabbyCtx,
    desc: *const CrabbyMaterialDesc,
    out_material: *mut CrabbyMaterial,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_create_material(ctx, desc, out_material) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_update_material`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_update_material(
    ctx: *mut CrabbyCtx,
    material: CrabbyMaterial,
    desc: *const CrabbyMaterialDesc,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_update_material(ctx, material, desc) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_loaded_materials`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_loaded_materials(
    ctx: *mut CrabbyCtx,
    name: *const c_char,
    out_materials: *mut CrabbyMaterial,
    out_descs: *mut CrabbyMaterialDesc,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_loaded_materials(ctx, name, out_materials, out_descs, capacity, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_load_shader`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_shader(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    out_shader: *mut CrabbyShader,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_load_shader(ctx, path, out_shader) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_create_shader`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_shader(
    ctx: *mut CrabbyCtx,
    name: *const c_char,
    source: *const c_char,
    out_shader: *mut CrabbyShader,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_create_shader(ctx, name, source, out_shader) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_load_compute`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_compute(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    out_compute: *mut CrabbyCompute,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_load_compute(ctx, path, out_compute) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_create_compute`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_compute(
    ctx: *mut CrabbyCtx,
    name: *const c_char,
    source: *const c_char,
    out_compute: *mut CrabbyCompute,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_create_compute(ctx, name, source, out_compute) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_compute_passes`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_compute_passes(
    ctx: *mut CrabbyCtx,
    passes: *const CrabbyComputePass,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_compute_passes(ctx, passes, count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_compute_passes`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_compute_passes(
    ctx: *mut CrabbyCtx,
    out_passes: *mut CrabbyComputePass,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_compute_passes(ctx, out_passes, capacity, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_load_effect`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_effect(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    out_effect: *mut CrabbyEffect,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_load_effect(ctx, path, out_effect) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_create_effect`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_effect(
    ctx: *mut CrabbyCtx,
    name: *const c_char,
    source: *const c_char,
    out_effect: *mut CrabbyEffect,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_create_effect(ctx, name, source, out_effect) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_effects`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_effects(
    ctx: *mut CrabbyCtx,
    effects: *const CrabbyScreenEffect,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_effects(ctx, effects, count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_effects`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_effects(
    ctx: *mut CrabbyCtx,
    out_effects: *mut CrabbyScreenEffect,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_effects(ctx, out_effects, capacity, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_default_material`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_default_material(
    ctx: *mut CrabbyCtx,
    out_material: *mut CrabbyMaterial,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_default_material(ctx, out_material) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_create_mesh`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_mesh(
    ctx: *mut CrabbyCtx,
    data: *const CrabbyMeshData,
    material: CrabbyMaterial,
    out_mesh: *mut CrabbyMesh,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_create_mesh(ctx, data, material, out_mesh) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_create_plain_mesh`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_plain_mesh(
    ctx: *mut CrabbyCtx,
    data: *const CrabbyMeshData,
    out_mesh: *mut CrabbyMesh,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_create_plain_mesh(ctx, data, out_mesh) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_create_terrain_tile`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_terrain_tile(
    ctx: *mut CrabbyCtx,
    heights: *const f32,
    count: usize,
    size: f32,
    material: CrabbyMaterial,
    out_tile: *mut CrabbyTerrainTile,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_create_terrain_tile(ctx, heights, count, size, material, out_tile) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_load_mesh`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_mesh(
    ctx: *mut CrabbyCtx,
    path: *const c_char,
    out_mesh: *mut CrabbyMesh,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_load_mesh(ctx, path, out_mesh) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_models_drawn`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_models_drawn(ctx: *mut CrabbyCtx, out_count: *mut usize) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_models_drawn(ctx, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_create_render_target`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_render_target(
    ctx: *mut CrabbyCtx,
    width: u32,
    height: u32,
    out_texture: *mut CrabbyTexture,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_create_render_target(ctx, width, height, out_texture) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_reflection_camera`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_reflection_camera(
    ctx: *mut CrabbyCtx,
    camera: *const CrabbyCamera3D,
    target: CrabbyTexture,
    level: f32,
    out_camera: *mut CrabbyTargetCamera,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_reflection_camera(ctx, camera, target, level, out_camera) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_target_cameras`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_target_cameras(
    ctx: *mut CrabbyCtx,
    cameras: *const CrabbyTargetCamera,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_target_cameras(ctx, cameras, count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_target_cameras`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_target_cameras(
    ctx: *mut CrabbyCtx,
    out_cameras: *mut CrabbyTargetCamera,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_target_cameras(ctx, out_cameras, capacity, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_camera3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_camera3d(ctx: *mut CrabbyCtx, out_camera: *mut CrabbyCamera3D) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_camera3d(ctx, out_camera) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_camera3d`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_camera3d(ctx: *mut CrabbyCtx, camera: *const CrabbyCamera3D) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_camera3d(ctx, camera) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_occlusion_culling`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_occlusion_culling(ctx: *mut CrabbyCtx, on: bool) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_occlusion_culling(ctx, on) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_occlusion_culling`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_occlusion_culling(ctx: *mut CrabbyCtx, out_on: *mut bool) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_occlusion_culling(ctx, out_on) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_camera3d_look_at`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_camera3d_look_at(ctx: *mut CrabbyCtx, target: CrabbyVec3) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_camera3d_look_at(ctx, target) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_screen_ray`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_screen_ray(
    ctx: *mut CrabbyCtx,
    screen: CrabbyVec2,
    out_origin: *mut CrabbyVec3,
    out_direction: *mut CrabbyVec3,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_screen_ray(ctx, screen, out_origin, out_direction) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_lighting`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_lighting(ctx: *mut CrabbyCtx, out_lighting: *mut CrabbyLighting) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_lighting(ctx, out_lighting) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_lighting`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_lighting(ctx: *mut CrabbyCtx, lighting: *const CrabbyLighting) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_lighting(ctx, lighting) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_sky`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_sky(ctx: *mut CrabbyCtx, out_sky: *mut CrabbySky) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_sky(ctx, out_sky) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_sky`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_sky(ctx: *mut CrabbyCtx, sky: *const CrabbySky) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_sky(ctx, sky) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_post`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_post(ctx: *mut CrabbyCtx, out_post: *mut CrabbyPost) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_post(ctx, out_post) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_post`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_post(ctx: *mut CrabbyCtx, post: *const CrabbyPost) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_post(ctx, post) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_graphics_quality_preset`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_graphics_quality_preset(
    level: u32,
    out_quality: *mut CrabbyGraphicsQuality,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_graphics_quality_preset(level, out_quality) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_graphics_quality_level`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_graphics_quality_level(
    quality: *const CrabbyGraphicsQuality,
    out_level: *mut u32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_graphics_quality_level(quality, out_level) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_graphics_quality`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_graphics_quality(
    ctx: *mut CrabbyCtx,
    out_quality: *mut CrabbyGraphicsQuality,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_graphics_quality(ctx, out_quality) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_graphics_quality`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_graphics_quality(
    ctx: *mut CrabbyCtx,
    quality: *const CrabbyGraphicsQuality,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_graphics_quality(ctx, quality) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_reset_temporal_history`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_reset_temporal_history(ctx: *mut CrabbyCtx) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_reset_temporal_history(ctx) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_heightfield_new`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_heightfield_new(
    heights: *const f32,
    count: usize,
    size: f32,
) -> *mut CrabbyHeightfield {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_heightfield_new(heights, count, size) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_heightfield_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_heightfield_free(ground: *mut CrabbyHeightfield) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_heightfield_free(ground) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_heightfield_at`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_heightfield_at(
    ground: *const CrabbyHeightfield,
    x: f32,
    z: f32,
    out_found: *mut bool,
    out_height: *mut f32,
    out_normal: *mut CrabbyVec3,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_heightfield_at(ground, x, z, out_found, out_height, out_normal) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_shadow_mesh`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_shadow_mesh(
    ctx: *mut CrabbyCtx,
    entity: u64,
    mesh: CrabbyMesh,
    casts: bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_shadow_mesh(ctx, entity, mesh, casts) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_model_material`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_model_material(
    ctx: *mut CrabbyCtx,
    entity: u64,
    material: CrabbyMaterial,
    enabled: bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_model_material(ctx, entity, material, enabled) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_model_material`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_model_material(
    ctx: *mut CrabbyCtx,
    entity: u64,
    out_material: *mut CrabbyMaterial,
    out_has: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_model_material(ctx, entity, out_material, out_has) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_model_part_materials`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_model_part_materials(
    ctx: *mut CrabbyCtx,
    entity: u64,
    parts: *const u32,
    materials: *const CrabbyMaterial,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_model_part_materials(ctx, entity, parts, materials, count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_model_part_materials`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_model_part_materials(
    ctx: *mut CrabbyCtx,
    entity: u64,
    out_parts: *mut u32,
    out_materials: *mut CrabbyMaterial,
    capacity: usize,
    out_count: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_model_part_materials(ctx, entity, out_parts, out_materials, capacity, out_count) })
        .into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_mesh_shadows`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_mesh_shadows(
    ctx: *mut CrabbyCtx,
    mesh: CrabbyMesh,
    casts: bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_mesh_shadows(ctx, mesh, casts) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_mesh_shadows`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_mesh_shadows(
    ctx: *mut CrabbyCtx,
    mesh: CrabbyMesh,
    out_casts: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_mesh_shadows(ctx, mesh, out_casts) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_point_light3d_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_point_light3d_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_point_light3d_component(world, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_spot_light3d_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_spot_light3d_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_spot_light3d_component(world, out_id) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_reflection_probe3d_default() -> CrabbyReflectionProbe3D {
    crate::ponko_reflection_probe3d_default()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_reflection_probe3d_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_reflection_probe3d_component(
    world: *mut CrabbyWorld,
    out_id: *mut u32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_reflection_probe3d_component(world, out_id) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_decal3d_default() -> CrabbyDecal3D {
    crate::ponko_decal3d_default()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_decal3d_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_decal3d_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_decal3d_component(world, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_receives_decals_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_receives_decals_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_receives_decals_component(world, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_decal_textures`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_decal_textures(ctx: *mut CrabbyCtx, array: CrabbyTextureArray) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_decal_textures(ctx, array) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_clear_decal_textures`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_clear_decal_textures(ctx: *mut CrabbyCtx) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_clear_decal_textures(ctx) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_supports_decals`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_supports_decals(ctx: *mut CrabbyCtx, out_supported: *mut bool) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_supports_decals(ctx, out_supported) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_warm_render_pipelines`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_warm_render_pipelines(ctx: *mut CrabbyCtx) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_warm_render_pipelines(ctx) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_bake_reflection_probe`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_bake_reflection_probe(ctx: *mut CrabbyCtx, entity: u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_bake_reflection_probe(ctx, entity) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_load_reflection_probe`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_load_reflection_probe(
    ctx: *mut CrabbyCtx,
    entity: u64,
    name: *const c_char,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_load_reflection_probe(ctx, entity, name) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_lod_renderer(mesh: CrabbyMesh) -> CrabbyLodRenderer {
    crate::ponko_lod_renderer(mesh)
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_lod_group`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_lod_group(
    ctx: *mut CrabbyCtx,
    entity: u64,
    size: f32,
    center: CrabbyVec3,
    levels: *const CrabbyLodLevel,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_lod_group(ctx, entity, size, center, levels, count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_lod_pick`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_lod_pick(
    ctx: *mut CrabbyCtx,
    entity: u64,
    out_pick: *mut CrabbyLodPick,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_lod_pick(ctx, entity, out_pick) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_lod_settings`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_lod_settings(
    ctx: *mut CrabbyCtx,
    out_settings: *mut CrabbyLodSettings,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_lod_settings(ctx, out_settings) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_lod_settings`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_lod_settings(ctx: *mut CrabbyCtx, settings: CrabbyLodSettings) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_lod_settings(ctx, settings) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_lod_screen_height_at(size: f32, distance: f32, fov_y: f32) -> f32 {
    crate::ponko_lod_screen_height_at(size, distance, fov_y)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_lod_distance_at(size: f32, height: f32, fov_y: f32) -> f32 {
    crate::ponko_lod_distance_at(size, height, fov_y)
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_create_gpu_buffer`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_gpu_buffer(
    ctx: *mut CrabbyCtx,
    len: u32,
    data: *const [f32; 4],
    count: usize,
    output: *mut CrabbyGpuBuffer,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_create_gpu_buffer(ctx, len, data, count, output) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_write_gpu_buffer`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_write_gpu_buffer(
    ctx: *mut CrabbyCtx,
    buffer: CrabbyGpuBuffer,
    offset: u32,
    data: *const [f32; 4],
    count: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_write_gpu_buffer(ctx, buffer, offset, data, count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_free_gpu_buffer`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_free_gpu_buffer(ctx: *mut CrabbyCtx, buffer: CrabbyGpuBuffer) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_free_gpu_buffer(ctx, buffer) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_mesh_gpu_bounds`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_mesh_gpu_bounds(
    ctx: *mut CrabbyCtx,
    mesh: CrabbyMesh,
    center: CrabbyVec3,
    radius: f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_mesh_gpu_bounds(ctx, mesh, center, radius) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_mesh_gpu_buffer`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_mesh_gpu_buffer(
    ctx: *mut CrabbyCtx,
    mesh: CrabbyMesh,
    buffer: CrabbyGpuBuffer,
    count: u32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_mesh_gpu_buffer(ctx, mesh, buffer, count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_instance_batch_culling`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_instance_batch_culling(
    ctx: *mut CrabbyCtx,
    batch: CrabbyInstanceBatch,
    culling: u32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_instance_batch_culling(ctx, batch, culling) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_create_instance_batch`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_instance_batch(
    ctx: *mut CrabbyCtx,
    mesh: CrabbyMesh,
    matrices: *const [f32; 16],
    count: usize,
    options: CrabbyInstanceBatchOptions,
    output: *mut CrabbyInstanceBatch,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_create_instance_batch(ctx, mesh, matrices, count, options, output) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_update_instance_batch`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_update_instance_batch(
    ctx: *mut CrabbyCtx,
    batch: CrabbyInstanceBatch,
    matrices: *const [f32; 16],
    count: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_update_instance_batch(ctx, batch, matrices, count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_free_instance_batch`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_free_instance_batch(
    ctx: *mut CrabbyCtx,
    batch: CrabbyInstanceBatch,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_free_instance_batch(ctx, batch) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_create_instance_batch_levels`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_instance_batch_levels(
    ctx: *mut CrabbyCtx,
    levels: *const CrabbyInstanceLevel,
    level_count: usize,
    matrices: *const [f32; 16],
    count: usize,
    colors: *const CrabbyColor,
    color_count: usize,
    options: CrabbyInstanceBatchOptions,
    output: *mut CrabbyInstanceBatch,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe {
        crate::ponko_ctx_create_instance_batch_levels(
            ctx,
            levels,
            level_count,
            matrices,
            count,
            colors,
            color_count,
            options,
            output,
        )
    })
    .into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_update_instance_batch_colors`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_update_instance_batch_colors(
    ctx: *mut CrabbyCtx,
    batch: CrabbyInstanceBatch,
    matrices: *const [f32; 16],
    count: usize,
    colors: *const CrabbyColor,
    color_count: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_update_instance_batch_colors(ctx, batch, matrices, count, colors, color_count) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_TILE_EMPTY as CRABBY_TILE_EMPTY;
/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_sprite_sheet`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_sprite_sheet(
    texture: CrabbyTexture,
    frame_width: u32,
    frame_height: u32,
    out_sheet: *mut CrabbySpriteSheet,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_sprite_sheet(texture, frame_width, frame_height, out_sheet) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_sprite_sheet_frame(sheet: CrabbySpriteSheet, index: u32) -> CrabbyRect {
    crate::ponko_sprite_sheet_frame(sheet, index)
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_animation_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_animation_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_animation_component(world, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_texture_filter`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_texture_filter(
    ctx: *mut CrabbyCtx,
    texture: CrabbyTexture,
    nearest: bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_texture_filter(ctx, texture, nearest) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_tilemap_add`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_tilemap_add(
    world: *mut CrabbyWorld,
    entity: u64,
    sheet: CrabbySpriteSheet,
    width: u32,
    height: u32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_tilemap_add(world, entity, sheet, width, height) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_tilemap_get`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_tilemap_get(
    world: *mut CrabbyWorld,
    entity: u64,
    out_map: *mut CrabbyTileMap,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_tilemap_get(world, entity, out_map) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_tilemap_update`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_tilemap_update(
    world: *mut CrabbyWorld,
    entity: u64,
    map: *const CrabbyTileMap,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_tilemap_update(world, entity, map) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_entity_bits(entity: CrabbyEntity) -> u64 {
    crate::ponko_entity_bits(entity)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_draw_params_default() -> CrabbyDrawParams {
    crate::ponko_draw_params_default()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_set_accessible_name`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_accessible_name(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    name: *const c_char,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_set_accessible_name(ctx, target, id, name) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_accessible_name`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_accessible_name(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_accessible_name(ctx, target, id, buffer, capacity, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_set_accessible_description`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_accessible_description(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    description: *const c_char,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_set_accessible_description(ctx, target, id, description) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_accessible_description`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_accessible_description(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_accessible_description(ctx, target, id, buffer, capacity, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_set_live_region`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_live_region(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    live: u32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_set_live_region(ctx, target, id, live) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_live_region`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_live_region(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    out_live: *mut CrabbyUiLive,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_live_region(ctx, target, id, out_live.cast()) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_access_action`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_access_action(
    ctx: *mut CrabbyCtx,
    target: u64,
    kind: u32,
    id: u64,
    value: f32,
    text: *const c_char,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_access_action(ctx, target, kind, id, value, text) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_access_text`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_access_text(
    ctx: *mut CrabbyCtx,
    target: u64,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_access_text(ctx, target, buffer, capacity, out_len) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ui_access_new() -> *mut CrabbyUiAccess {
    crate::ponko_ui_access_new()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ui_access_free`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ui_access_free(access: *mut CrabbyUiAccess) {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_ui_access_free(access) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ui_access_update`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ui_access_update(
    access: *mut CrabbyUiAccess,
    ctx: *mut CrabbyCtx,
    target: u64,
    out_changes: *mut CrabbyUiAccessChanges,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ui_access_update(access, ctx, target, out_changes) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ui_access_reset`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ui_access_reset(access: *mut CrabbyUiAccess) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ui_access_reset(access) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ui_access_count`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ui_access_count(access: *mut CrabbyUiAccess, out_count: *mut usize) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ui_access_count(access, out_count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ui_access_node`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ui_access_node(
    access: *mut CrabbyUiAccess,
    index: usize,
    out_node: *mut CrabbyUiAccessNode,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ui_access_node(access, index, out_node.cast()) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ui_access_find`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ui_access_find(
    access: *mut CrabbyUiAccess,
    id: u64,
    out_node: *mut CrabbyUiAccessNode,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ui_access_find(access, id, out_node.cast()) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ui_access_child`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ui_access_child(
    access: *mut CrabbyUiAccess,
    id: u64,
    index: usize,
    out_child: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ui_access_child(access, id, index, out_child) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ui_access_name`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ui_access_name(
    access: *mut CrabbyUiAccess,
    id: u64,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ui_access_name(access, id, buffer, capacity, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ui_access_description`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ui_access_description(
    access: *mut CrabbyUiAccess,
    id: u64,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ui_access_description(access, id, buffer, capacity, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ui_access_value_text`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ui_access_value_text(
    access: *mut CrabbyUiAccess,
    id: u64,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ui_access_value_text(access, id, buffer, capacity, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ui_access_focus`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ui_access_focus(access: *mut CrabbyUiAccess, out_id: *mut u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ui_access_focus(access, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ui_access_changed`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ui_access_changed(
    access: *mut CrabbyUiAccess,
    index: usize,
    out_id: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ui_access_changed(access, index, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ui_access_removed`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ui_access_removed(
    access: *mut CrabbyUiAccess,
    index: usize,
    out_id: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ui_access_removed(access, index, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_create_ui_panel`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_create_ui_panel(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    width: u32,
    height: u32,
    size: CrabbyVec2,
    out_texture: *mut CrabbyTexture,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_create_ui_panel(ctx, entity_bits, width, height, size, out_texture) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_remove_ui_panel`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_remove_ui_panel(ctx: *mut CrabbyCtx, entity_bits: u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_remove_ui_panel(ctx, entity_bits) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_ui_panel_interactive`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_ui_panel_interactive(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    interactive: bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_ui_panel_interactive(ctx, entity_bits, interactive) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_panel_pointer`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_panel_pointer(
    ctx: *mut CrabbyCtx,
    entity_bits: u64,
    out_over: *mut bool,
    out_pixel: *mut CrabbyVec2,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_panel_pointer(ctx, entity_bits, out_over, out_pixel) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_set_ui_pointer_ray`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_set_ui_pointer_ray(
    ctx: *mut CrabbyCtx,
    origin: CrabbyVec3,
    direction: CrabbyVec3,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_set_ui_pointer_ray(ctx, origin, direction) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_clear_ui_pointer_ray`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_clear_ui_pointer_ray(ctx: *mut CrabbyCtx) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_clear_ui_pointer_ray(ctx) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_UI_NONE as CRABBY_UI_NONE;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
pub use crate::PONKO_UI_SCREEN as CRABBY_UI_SCREEN;
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ui_layout_default() -> CrabbyUiLayout {
    crate::ponko_ui_layout_default()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ui_paint_default() -> CrabbyUiPaint {
    crate::ponko_ui_paint_default()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ui_input_default() -> CrabbyUiInput {
    crate::ponko_ui_input_default()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_add_panel`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_add_panel(
    ctx: *mut CrabbyCtx,
    target: u64,
    parent: u64,
    layout: *const CrabbyUiLayout,
    out_id: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_add_panel(ctx, target, parent, layout, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_add_label`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_add_label(
    ctx: *mut CrabbyCtx,
    target: u64,
    parent: u64,
    text: *const c_char,
    layout: *const CrabbyUiLayout,
    out_id: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_add_label(ctx, target, parent, text, layout, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_add_button`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_add_button(
    ctx: *mut CrabbyCtx,
    target: u64,
    parent: u64,
    text: *const c_char,
    layout: *const CrabbyUiLayout,
    out_id: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_add_button(ctx, target, parent, text, layout, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_add_image`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_add_image(
    ctx: *mut CrabbyCtx,
    target: u64,
    parent: u64,
    texture: CrabbyTexture,
    layout: *const CrabbyUiLayout,
    out_id: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_add_image(ctx, target, parent, texture, layout, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_add_scroll`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_add_scroll(
    ctx: *mut CrabbyCtx,
    target: u64,
    parent: u64,
    layout: *const CrabbyUiLayout,
    out_id: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_add_scroll(ctx, target, parent, layout, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_add_text_input`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_add_text_input(
    ctx: *mut CrabbyCtx,
    target: u64,
    parent: u64,
    text: *const c_char,
    layout: *const CrabbyUiLayout,
    out_id: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_add_text_input(ctx, target, parent, text, layout, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_add_toggle`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_add_toggle(
    ctx: *mut CrabbyCtx,
    target: u64,
    parent: u64,
    text: *const c_char,
    on: bool,
    layout: *const CrabbyUiLayout,
    out_id: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_add_toggle(ctx, target, parent, text, on, layout, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_add_slider`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_add_slider(
    ctx: *mut CrabbyCtx,
    target: u64,
    parent: u64,
    value: f32,
    layout: *const CrabbyUiLayout,
    out_id: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_add_slider(ctx, target, parent, value, layout, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_remove`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_remove(ctx: *mut CrabbyCtx, target: u64, id: u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_remove(ctx, target, id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_set_layout`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_layout(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    layout: *const CrabbyUiLayout,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_set_layout(ctx, target, id, layout) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_set_grid_template_areas`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_grid_template_areas(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    rows: *const *const c_char,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_set_grid_template_areas(ctx, target, id, rows, count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_set_grid_line_names`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_grid_line_names(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    axis: u32,
    names: *const *const c_char,
    count: usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_set_grid_line_names(ctx, target, id, axis, names, count) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_set_grid_area`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_grid_area(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    name: *const c_char,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_set_grid_area(ctx, target, id, name) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_set_grid_named_line`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_grid_named_line(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    axis: u32,
    end: bool,
    name: *const c_char,
    occurrence: i32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_set_grid_named_line(ctx, target, id, axis, end, name, occurrence) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_set_paint`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_paint(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    paint: *const CrabbyUiPaint,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_set_paint(ctx, target, id, paint) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_set_text`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_text(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    text: *const c_char,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_set_text(ctx, target, id, text) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_text`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_text(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_text(ctx, target, id, buffer, capacity, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_set_enabled`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_enabled(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    enabled: bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_set_enabled(ctx, target, id, enabled) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_set_value`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_value(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    value: f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_set_value(ctx, target, id, value) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_value`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_value(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    out_value: *mut f32,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_value(ctx, target, id, out_value) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_set_scroll`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_scroll(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    offset: CrabbyVec2,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_set_scroll(ctx, target, id, offset) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_scroll`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_scroll(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    out_offset: *mut CrabbyVec2,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_scroll(ctx, target, id, out_offset) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_push_screen`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_push_screen(
    ctx: *mut CrabbyCtx,
    target: u64,
    root: u64,
    modal: bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_push_screen(ctx, target, root, modal) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_pop_screen`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_pop_screen(
    ctx: *mut CrabbyCtx,
    target: u64,
    out_root: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_pop_screen(ctx, target, out_root) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_top_screen`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_top_screen(
    ctx: *mut CrabbyCtx,
    target: u64,
    out_root: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_top_screen(ctx, target, out_root) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_focus`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_focus(ctx: *mut CrabbyCtx, target: u64, out_id: *mut u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_focus(ctx, target, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_set_scale`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_scale(ctx: *mut CrabbyCtx, target: u64, scale: f32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_set_scale(ctx, target, scale) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_rect`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_rect(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    out_rect: *mut CrabbyRect,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_rect(ctx, target, id, out_rect) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_hit_test`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_hit_test(
    ctx: *mut CrabbyCtx,
    target: u64,
    point: CrabbyVec2,
    out_id: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_hit_test(ctx, target, point, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_wants_pointer`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_wants_pointer(
    ctx: *mut CrabbyCtx,
    target: u64,
    out_wanted: *mut bool,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_wants_pointer(ctx, target, out_wanted) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_wants_keyboard`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_wants_keyboard(ctx: *mut CrabbyCtx, out_wanted: *mut bool) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_wants_keyboard(ctx, out_wanted) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_next_event`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_next_event(
    ctx: *mut CrabbyCtx,
    target: u64,
    out_event: *mut CrabbyUiEvent,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_next_event(ctx, target, out_event.cast()) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_layout`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_layout(ctx: *mut CrabbyCtx, target: u64, viewport: CrabbyVec2) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_layout(ctx, target, viewport) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_frame`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_frame(
    ctx: *mut CrabbyCtx,
    target: u64,
    viewport: CrabbyVec2,
    input: *const CrabbyUiInput,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_frame(ctx, target, viewport, input) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_insert_text`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_insert_text(ctx: *mut CrabbyCtx, text: *const c_char) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_insert_text(ctx, text) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ui_layout_row(height: f32) -> CrabbyUiLayout {
    crate::ponko_ui_layout_row(height)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ui_layout_column(width: f32) -> CrabbyUiLayout {
    crate::ponko_ui_layout_column(width)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ui_layout_fill() -> CrabbyUiLayout {
    crate::ponko_ui_layout_fill()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ui_layout_fixed(width: f32, height: f32) -> CrabbyUiLayout {
    crate::ponko_ui_layout_fixed(width, height)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ui_layout_anchored(x: f32, y: f32, width: f32, height: f32) -> CrabbyUiLayout {
    crate::ponko_ui_layout_anchored(x, y, width, height)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ui_layout_centered(width: f32, height: f32) -> CrabbyUiLayout {
    crate::ponko_ui_layout_centered(width, height)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_ui_theme_default() -> CrabbyUiTheme {
    crate::ponko_ui_theme_default()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_theme`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_theme(
    ctx: *mut CrabbyCtx,
    target: u64,
    out_theme: *mut CrabbyUiTheme,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_theme(ctx, target, out_theme) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_set_theme`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_theme(
    ctx: *mut CrabbyCtx,
    target: u64,
    theme: *const CrabbyUiTheme,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_set_theme(ctx, target, theme) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_paint`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_paint(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    out_paint: *mut CrabbyUiPaint,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_paint(ctx, target, id, out_paint) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_add_title`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_add_title(
    ctx: *mut CrabbyCtx,
    target: u64,
    parent: u64,
    text: *const c_char,
    layout: *const CrabbyUiLayout,
    out_id: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_add_title(ctx, target, parent, text, layout, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_set_name`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_set_name(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    name: *const c_char,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_set_name(ctx, target, id, name) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_find`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_find(
    ctx: *mut CrabbyCtx,
    target: u64,
    name: *const c_char,
    out_id: *mut u64,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_find(ctx, target, name, out_id) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_name`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_name(
    ctx: *mut CrabbyCtx,
    target: u64,
    id: u64,
    buffer: *mut c_char,
    capacity: usize,
    out_len: *mut usize,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_name(ctx, target, id, buffer, capacity, out_len) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_ctx_ui_take_click`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_ctx_ui_take_click(ctx: *mut CrabbyCtx, target: u64, out_id: *mut u64) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_ctx_ui_take_click(ctx, target, out_id) }).into()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_friction_curve_forward() -> CrabbyFrictionCurve {
    crate::ponko_friction_curve_forward()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_friction_curve_sideways() -> CrabbyFrictionCurve {
    crate::ponko_friction_curve_sideways()
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_friction_curve_evaluate(curve: CrabbyFrictionCurve, slip: f32) -> f32 {
    crate::ponko_friction_curve_evaluate(curve, slip)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_wheel3d(position: CrabbyVec3, radius: f32) -> CrabbyWheel3D {
    crate::ponko_wheel3d(position, radius)
}

#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub extern "C" fn crabby_vehicle3d() -> CrabbyVehicle3D {
    crate::ponko_vehicle3d()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_vehicle3d_add_wheel`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_vehicle3d_add_wheel(
    vehicle: *mut CrabbyVehicle3D,
    wheel: CrabbyWheel3D,
) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_vehicle3d_add_wheel(vehicle, wheel) }).into()
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_wheel3d_pose`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_wheel3d_pose(
    wheel: *const CrabbyWheel3D,
    chassis: CrabbyTransform3D,
) -> CrabbyTransform3D {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    unsafe { crate::ponko_wheel3d_pose(wheel, chassis) }
}

/// # Safety
/// The caller must uphold the same preconditions as [`crate::ponko_vehicle3d_component`].
#[deprecated(
    note = "Deprecated: use the Ponko spelling. Removal requires a major release, no earlier than 2027-11-10."
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crabby_vehicle3d_component(world: *mut CrabbyWorld, out_id: *mut u32) -> CrabbyStatus {
    // SAFETY: ABI-2 forwarding preserves the primary function's exact
    // pointer, lifetime, ownership and callback preconditions. Distinct value
    // types have identical repr(C) layouts and valid discriminants, checked above.
    (unsafe { crate::ponko_vehicle3d_component(world, out_id) }).into()
}
