/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use cssparser::{Parser, ParserInput};
use cssparser::ToCss as ParserToCss;
use env_logger::LogBuilder;
use selectors::Element;
use selectors::matching::{MatchingContext, MatchingMode, matches_selector};
use servo_arc::{Arc, ArcBorrow, RawOffsetArc};
use std::env;
use std::fmt::Write;
use std::ptr;
use style::context::{CascadeInputs, QuirksMode, SharedStyleContext, StyleContext};
use style::context::ThreadLocalStyleContext;
use style::data::ElementStyles;
use style::dom::{ShowSubtreeData, TElement, TNode};
use style::element_state::ElementState;
use style::error_reporting::{NullReporter, ParseErrorReporter};
use style::font_metrics::{FontMetricsProvider, get_metrics_provider_for_product};
use style::gecko::data::{GeckoStyleSheet, PerDocumentStyleData, PerDocumentStyleDataImpl};
use style::gecko::global_style_data::{GLOBAL_STYLE_DATA, GlobalStyleData, STYLE_THREAD_POOL};
use style::gecko::restyle_damage::GeckoRestyleDamage;
use style::gecko::selector_parser::PseudoElement;
use style::gecko::traversal::RecalcStyleOnly;
use style::gecko::wrapper::GeckoElement;
use style::gecko_bindings::bindings::{RawGeckoElementBorrowed, RawGeckoElementBorrowedOrNull};
use style::gecko_bindings::bindings::{RawGeckoKeyframeListBorrowed, RawGeckoKeyframeListBorrowedMut};
use style::gecko_bindings::bindings::{RawServoDeclarationBlockBorrowed, RawServoDeclarationBlockStrong};
use style::gecko_bindings::bindings::{RawServoDocumentRule, RawServoDocumentRuleBorrowed};
use style::gecko_bindings::bindings::{RawServoFontFeatureValuesRule, RawServoFontFeatureValuesRuleBorrowed};
use style::gecko_bindings::bindings::{RawServoImportRule, RawServoImportRuleBorrowed};
use style::gecko_bindings::bindings::{RawServoKeyframe, RawServoKeyframeBorrowed, RawServoKeyframeStrong};
use style::gecko_bindings::bindings::{RawServoKeyframesRule, RawServoKeyframesRuleBorrowed};
use style::gecko_bindings::bindings::{RawServoMediaListBorrowed, RawServoMediaListStrong};
use style::gecko_bindings::bindings::{RawServoMediaRule, RawServoMediaRuleBorrowed};
use style::gecko_bindings::bindings::{RawServoNamespaceRule, RawServoNamespaceRuleBorrowed};
use style::gecko_bindings::bindings::{RawServoPageRule, RawServoPageRuleBorrowed};
use style::gecko_bindings::bindings::{RawServoStyleSetBorrowed, RawServoStyleSetOwned};
use style::gecko_bindings::bindings::{RawServoStyleSheetContentsBorrowed, ServoComputedDataBorrowed};
use style::gecko_bindings::bindings::{RawServoStyleSheetContentsStrong, ServoStyleContextBorrowed};
use style::gecko_bindings::bindings::{RawServoSupportsRule, RawServoSupportsRuleBorrowed};
use style::gecko_bindings::bindings::{ServoCssRulesBorrowed, ServoCssRulesStrong};
use style::gecko_bindings::bindings::{nsACString, nsAString, nsCSSPropertyIDSetBorrowedMut};
use style::gecko_bindings::bindings::Gecko_AddPropertyToSet;
use style::gecko_bindings::bindings::Gecko_AppendPropertyValuePair;
use style::gecko_bindings::bindings::Gecko_GetOrCreateFinalKeyframe;
use style::gecko_bindings::bindings::Gecko_GetOrCreateInitialKeyframe;
use style::gecko_bindings::bindings::Gecko_GetOrCreateKeyframeAtStart;
use style::gecko_bindings::bindings::Gecko_NewNoneTransform;
use style::gecko_bindings::bindings::RawGeckoAnimationPropertySegmentBorrowed;
use style::gecko_bindings::bindings::RawGeckoCSSPropertyIDListBorrowed;
use style::gecko_bindings::bindings::RawGeckoComputedKeyframeValuesListBorrowedMut;
use style::gecko_bindings::bindings::RawGeckoComputedTimingBorrowed;
use style::gecko_bindings::bindings::RawGeckoFontFaceRuleListBorrowedMut;
use style::gecko_bindings::bindings::RawGeckoServoAnimationValueListBorrowed;
use style::gecko_bindings::bindings::RawGeckoServoAnimationValueListBorrowedMut;
use style::gecko_bindings::bindings::RawGeckoServoStyleRuleListBorrowedMut;
use style::gecko_bindings::bindings::RawServoAnimationValueBorrowed;
use style::gecko_bindings::bindings::RawServoAnimationValueMapBorrowedMut;
use style::gecko_bindings::bindings::RawServoAnimationValueStrong;
use style::gecko_bindings::bindings::RawServoAnimationValueTableBorrowed;
use style::gecko_bindings::bindings::RawServoStyleRuleBorrowed;
use style::gecko_bindings::bindings::ServoStyleContextBorrowedOrNull;
use style::gecko_bindings::bindings::nsTArrayBorrowed_uintptr_t;
use style::gecko_bindings::bindings::nsTimingFunctionBorrowed;
use style::gecko_bindings::bindings::nsTimingFunctionBorrowedMut;
use style::gecko_bindings::structs;
use style::gecko_bindings::structs::{CSSPseudoElementType, CompositeOperation, Loader};
use style::gecko_bindings::structs::{RawServoStyleRule, ServoStyleContextStrong};
use style::gecko_bindings::structs::{ServoStyleSheet, SheetParsingMode, nsIAtom, nsCSSPropertyID};
use style::gecko_bindings::structs::{nsCSSFontFaceRule, nsCSSCounterStyleRule};
use style::gecko_bindings::structs::{nsRestyleHint, nsChangeHint, PropertyValuePair};
use style::gecko_bindings::structs::IterationCompositeOperation;
use style::gecko_bindings::structs::MallocSizeOf;
use style::gecko_bindings::structs::RawGeckoGfxMatrix4x4;
use style::gecko_bindings::structs::RawGeckoPresContextOwned;
use style::gecko_bindings::structs::SeenPtrs;
use style::gecko_bindings::structs::ServoElementSnapshotTable;
use style::gecko_bindings::structs::ServoTraversalFlags;
use style::gecko_bindings::structs::StyleRuleInclusion;
use style::gecko_bindings::structs::URLExtraData;
use style::gecko_bindings::structs::nsCSSValueSharedList;
use style::gecko_bindings::structs::nsCompatibility;
use style::gecko_bindings::structs::nsIDocument;
use style::gecko_bindings::structs::nsStyleTransformMatrix::MatrixTransformOperator;
use style::gecko_bindings::structs::nsTArray;
use style::gecko_bindings::structs::nsresult;
use style::gecko_bindings::sugar::ownership::{FFIArcHelpers, HasFFI, HasArcFFI, HasBoxFFI};
use style::gecko_bindings::sugar::ownership::{HasSimpleFFI, Strong};
use style::gecko_bindings::sugar::refptr::RefPtr;
use style::gecko_properties::style_structs;
use style::invalidation::element::restyle_hints;
use style::media_queries::{MediaList, parse_media_query_list};
use style::parallel;
use style::parser::{ParserContext, self};
use style::properties::{CascadeFlags, ComputedValues, Importance};
use style::properties::{IS_FIELDSET_CONTENT, IS_LINK, IS_VISITED_LINK, LonghandIdSet};
use style::properties::{PropertyDeclaration, PropertyDeclarationBlock, PropertyId, ShorthandId};
use style::properties::{SKIP_ROOT_AND_ITEM_BASED_DISPLAY_FIXUP, SourcePropertyDeclaration, StyleBuilder};
use style::properties::PROHIBIT_DISPLAY_CONTENTS;
use style::properties::animated_properties::{Animatable, AnimatableLonghand, AnimationValue};
use style::properties::animated_properties::compare_property_priority;
use style::properties::parse_one_declaration_into;
use style::rule_tree::StyleSource;
use style::selector_parser::PseudoElementCascadeType;
use style::sequential;
use style::shared_lock::{SharedRwLockReadGuard, StylesheetGuards, ToCssWithGuard, Locked};
use style::string_cache::Atom;
use style::style_adjuster::StyleAdjuster;
use style::stylesheets::{CssRule, CssRules, CssRuleType, CssRulesHelpers, DocumentRule};
use style::stylesheets::{FontFeatureValuesRule, ImportRule, KeyframesRule, MallocSizeOfWithGuard};
use style::stylesheets::{MallocSizeOfWithRepeats, MediaRule};
use style::stylesheets::{NamespaceRule, Origin, PageRule, SizeOfState, StyleRule, SupportsRule};
use style::stylesheets::StylesheetContents;
use style::stylesheets::StylesheetLoader as StyleStylesheetLoader;
use style::stylesheets::keyframes_rule::{Keyframe, KeyframeSelector, KeyframesStepValue};
use style::stylesheets::supports_rule::parse_condition_or_declaration;
use style::stylist::RuleInclusion;
use style::thread_state;
use style::timer::Timer;
use style::traversal::{DomTraversal, TraversalDriver};
use style::traversal::resolve_style;
use style::traversal_flags::{TraversalFlags, self};
use style::values::{CustomIdent, KeyframesName};
use style::values::animated::ToAnimatedZero;
use style::values::computed::Context;
use style_traits::{PARSING_MODE_DEFAULT, ToCss};
use super::error_reporter::ErrorReporter;
use super::stylesheet_loader::StylesheetLoader;

/*
 * For Gecko->Servo function calls, we need to redeclare the same signature that was declared in
 * the C header in Gecko. In order to catch accidental mismatches, we run rust-bindgen against
 * those signatures as well, giving us a second declaration of all the Servo_* functions in this
 * crate. If there's a mismatch, LLVM will assert and abort, which is a rather awful thing to
 * depend on but good enough for our purposes.
 */

// A dummy url data for where we don't pass url data in.
// We need to get rid of this sooner than later.
static mut DUMMY_URL_DATA: *mut URLExtraData = 0 as *mut URLExtraData;

#[no_mangle]
pub extern "C" fn Servo_Initialize(dummy_url_data: *mut URLExtraData) {
    // Initialize logging.
    let mut builder = LogBuilder::new();
    let default_level = if cfg!(debug_assertions) { "warn" } else { "error" };
    match env::var("RUST_LOG") {
      Ok(v) => builder.parse(&v).init().unwrap(),
      _ => builder.parse(default_level).init().unwrap(),
    };

    // Pretend that we're a Servo Layout thread, to make some assertions happy.
    thread_state::initialize(thread_state::LAYOUT);

    // Perform some debug-only runtime assertions.
    restyle_hints::assert_restyle_hints_match();
    parser::assert_parsing_mode_match();
    traversal_flags::assert_traversal_flags_match();

    // Initialize the dummy url data
    unsafe { DUMMY_URL_DATA = dummy_url_data; }
}

#[no_mangle]
pub extern "C" fn Servo_Shutdown() {
    // The dummy url will be released after shutdown, so clear the
    // reference to avoid use-after-free.
    unsafe { DUMMY_URL_DATA = ptr::null_mut(); }
}

unsafe fn dummy_url_data() -> &'static RefPtr<URLExtraData> {
    RefPtr::from_ptr_ref(&DUMMY_URL_DATA)
}

fn create_shared_context<'a>(global_style_data: &GlobalStyleData,
                             guard: &'a SharedRwLockReadGuard,
                             per_doc_data: &'a PerDocumentStyleDataImpl,
                             traversal_flags: TraversalFlags,
                             snapshot_map: &'a ServoElementSnapshotTable)
                             -> SharedStyleContext<'a> {
    SharedStyleContext {
        stylist: &per_doc_data.stylist,
        visited_styles_enabled: per_doc_data.visited_styles_enabled(),
        options: global_style_data.options.clone(),
        guards: StylesheetGuards::same(guard),
        timer: Timer::new(),
        quirks_mode: per_doc_data.stylist.quirks_mode(),
        traversal_flags: traversal_flags,
        snapshot_map: snapshot_map,
    }
}

fn traverse_subtree(element: GeckoElement,
                    raw_data: RawServoStyleSetBorrowed,
                    traversal_flags: TraversalFlags,
                    snapshots: &ServoElementSnapshotTable) {
    // When new content is inserted in a display:none subtree, we will call into
    // servo to try to style it. Detect that here and bail out.
    if let Some(parent) = element.traversal_parent() {
        if parent.borrow_data().map_or(true, |d| d.styles.is_display_none()) {
            debug!("{:?} has unstyled parent {:?} - ignoring call to traverse_subtree", element, parent);
            return;
        }
    }

    let per_doc_data = PerDocumentStyleData::from_ffi(raw_data).borrow();
    debug_assert!(!per_doc_data.stylesheets.has_changed());

    let global_style_data = &*GLOBAL_STYLE_DATA;
    let guard = global_style_data.shared_lock.read();
    let shared_style_context = create_shared_context(&global_style_data,
                                                     &guard,
                                                     &per_doc_data,
                                                     traversal_flags,
                                                     snapshots);


    let token = RecalcStyleOnly::pre_traverse(element,
                                              &shared_style_context,
                                              traversal_flags);
    if !token.should_traverse() {
        return;
    }

    debug!("Traversing subtree:");
    debug!("{:?}", ShowSubtreeData(element.as_node()));

    let style_thread_pool = &*STYLE_THREAD_POOL;
    let traversal_driver = if style_thread_pool.style_thread_pool.is_none() || !element.is_root() {
        TraversalDriver::Sequential
    } else {
        TraversalDriver::Parallel
    };

    let traversal = RecalcStyleOnly::new(shared_style_context, traversal_driver);
    if traversal_driver.is_parallel() {
        parallel::traverse_dom(&traversal, element, token,
                               style_thread_pool.style_thread_pool.as_ref().unwrap());
    } else {
        sequential::traverse_dom(&traversal, element, token);
    }
}

/// Traverses the subtree rooted at `root` for restyling.
///
/// Returns whether a Gecko post-traversal (to perform lazy frame construction,
/// or consume any RestyleData, or drop any ElementData) is required.
#[no_mangle]
pub extern "C" fn Servo_TraverseSubtree(root: RawGeckoElementBorrowed,
                                        raw_data: RawServoStyleSetBorrowed,
                                        snapshots: *const ServoElementSnapshotTable,
                                        raw_flags: ServoTraversalFlags)
                                        -> bool {
    let traversal_flags = TraversalFlags::from_bits_truncate(raw_flags);
    debug_assert!(!snapshots.is_null());

    let element = GeckoElement(root);

    // It makes no sense to do an animation restyle when we're restyling
    // newly-inserted content.
    if !traversal_flags.contains(traversal_flags::UnstyledChildrenOnly) {
        let needs_animation_only_restyle =
            element.has_animation_only_dirty_descendants() ||
            element.has_animation_restyle_hints();

        if needs_animation_only_restyle {
            traverse_subtree(element,
                             raw_data,
                             traversal_flags | traversal_flags::AnimationOnly,
                             unsafe { &*snapshots });
        }
    }

    if traversal_flags.for_animation_only() {
        return element.has_animation_only_dirty_descendants() ||
               element.borrow_data().unwrap().restyle.is_restyle();
    }

    traverse_subtree(element,
                     raw_data,
                     traversal_flags,
                     unsafe { &*snapshots });

    element.has_dirty_descendants() ||
    element.has_animation_only_dirty_descendants() ||
    element.borrow_data().unwrap().restyle.contains_restyle_data()
}

/// Checks whether the rule tree has crossed its threshold for unused nodes, and
/// if so, frees them.
#[no_mangle]
pub extern "C" fn Servo_MaybeGCRuleTree(raw_data: RawServoStyleSetBorrowed) {
    let per_doc_data = PerDocumentStyleData::from_ffi(raw_data).borrow_mut();
    unsafe {
        per_doc_data.stylist.rule_tree().maybe_gc();
    }
}

#[no_mangle]
pub extern "C" fn Servo_AnimationValues_Interpolate(from: RawServoAnimationValueBorrowed,
                                                    to: RawServoAnimationValueBorrowed,
                                                    progress: f64)
     -> RawServoAnimationValueStrong
{
    let from_value = AnimationValue::as_arc(&from);
    let to_value = AnimationValue::as_arc(&to);
    if let Ok(value) = from_value.interpolate(to_value, progress) {
        Arc::new(value).into_strong()
    } else {
        RawServoAnimationValueStrong::null()
    }
}

#[no_mangle]
pub extern "C" fn Servo_AnimationValues_IsInterpolable(from: RawServoAnimationValueBorrowed,
                                                       to: RawServoAnimationValueBorrowed)
                                                       -> bool {
    let from_value = AnimationValue::as_arc(&from);
    let to_value = AnimationValue::as_arc(&to);
    from_value.interpolate(to_value, 0.5).is_ok()
}

#[no_mangle]
pub extern "C" fn Servo_AnimationValues_Add(a: RawServoAnimationValueBorrowed,
                                            b: RawServoAnimationValueBorrowed)
     -> RawServoAnimationValueStrong
{
    let a_value = AnimationValue::as_arc(&a);
    let b_value = AnimationValue::as_arc(&b);
    if let Ok(value) = a_value.add(b_value) {
        Arc::new(value).into_strong()
    } else {
        RawServoAnimationValueStrong::null()
    }
}

#[no_mangle]
pub extern "C" fn Servo_AnimationValues_Accumulate(a: RawServoAnimationValueBorrowed,
                                                   b: RawServoAnimationValueBorrowed,
                                                   count: u64)
     -> RawServoAnimationValueStrong
{
    let a_value = AnimationValue::as_arc(&a);
    let b_value = AnimationValue::as_arc(&b);
    if let Ok(value) = a_value.accumulate(b_value, count) {
        Arc::new(value).into_strong()
    } else {
        RawServoAnimationValueStrong::null()
    }
}

#[no_mangle]
pub extern "C" fn Servo_AnimationValues_GetZeroValue(
    value_to_match: RawServoAnimationValueBorrowed)
    -> RawServoAnimationValueStrong
{
    let value_to_match = AnimationValue::as_arc(&value_to_match);
    if let Ok(zero_value) = value_to_match.to_animated_zero() {
        Arc::new(zero_value).into_strong()
    } else {
        RawServoAnimationValueStrong::null()
    }
}

#[no_mangle]
pub extern "C" fn Servo_AnimationValues_ComputeDistance(from: RawServoAnimationValueBorrowed,
                                                        to: RawServoAnimationValueBorrowed)
                                                        -> f64 {
    let from_value = AnimationValue::as_arc(&from);
    let to_value = AnimationValue::as_arc(&to);
    from_value.compute_distance(to_value).unwrap_or(0.0)
}

#[no_mangle]
pub extern "C" fn Servo_AnimationCompose(raw_value_map: RawServoAnimationValueMapBorrowedMut,
                                         base_values: RawServoAnimationValueTableBorrowed,
                                         css_property: nsCSSPropertyID,
                                         segment: RawGeckoAnimationPropertySegmentBorrowed,
                                         last_segment: RawGeckoAnimationPropertySegmentBorrowed,
                                         computed_timing: RawGeckoComputedTimingBorrowed,
                                         iteration_composite: IterationCompositeOperation)
{
    use style::gecko_bindings::bindings::Gecko_AnimationGetBaseStyle;
    use style::gecko_bindings::bindings::Gecko_GetPositionInSegment;
    use style::gecko_bindings::bindings::Gecko_GetProgressFromComputedTiming;
    use style::properties::animated_properties::AnimationValueMap;

    let property = match AnimatableLonghand::from_nscsspropertyid(css_property) {
        Some(longhand) => longhand,
        None => { return (); }
    };
    let value_map = AnimationValueMap::from_ffi_mut(raw_value_map);

    // We will need an underlying value if either of the endpoints is null...
    let need_underlying_value = segment.mFromValue.mServo.mRawPtr.is_null() ||
                                segment.mToValue.mServo.mRawPtr.is_null() ||
                                // ... or if they have a non-replace composite mode ...
                                segment.mFromComposite != CompositeOperation::Replace ||
                                segment.mToComposite != CompositeOperation::Replace ||
                                // ... or if we accumulate onto the last value and it is null.
                                (iteration_composite == IterationCompositeOperation::Accumulate &&
                                 computed_timing.mCurrentIteration > 0 &&
                                 last_segment.mToValue.mServo.mRawPtr.is_null());

    // If either of the segment endpoints are null, get the underlying value to
    // use from the current value in the values map (set by a lower-priority
    // effect), or, if there is no current value, look up the cached base value
    // for this property.
    let underlying_value = if need_underlying_value {
        let previous_composed_value = value_map.get(&property).cloned();
        previous_composed_value.or_else(|| {
            let raw_base_style = unsafe { Gecko_AnimationGetBaseStyle(base_values, css_property) };
            AnimationValue::arc_from_borrowed(&raw_base_style).map(|v| &**v).cloned()
        })
    } else {
        None
    };

    if need_underlying_value && underlying_value.is_none() {
        warn!("Underlying value should be valid when we expect to use it");
        return;
    }

    // Extract keyframe values.
    let raw_from_value;
    let keyframe_from_value = if !segment.mFromValue.mServo.mRawPtr.is_null() {
        raw_from_value = unsafe { &*segment.mFromValue.mServo.mRawPtr };
        Some(AnimationValue::as_arc(&raw_from_value))
    } else {
        None
    };

    let raw_to_value;
    let keyframe_to_value = if !segment.mToValue.mServo.mRawPtr.is_null() {
        raw_to_value = unsafe { &*segment.mToValue.mServo.mRawPtr };
        Some(AnimationValue::as_arc(&raw_to_value))
    } else {
        None
    };

    // Composite with underlying value.
    // A return value of None means, "Just use keyframe_value as-is."
    let composite_endpoint = |keyframe_value: Option<&RawOffsetArc<AnimationValue>>,
                              composite_op: CompositeOperation| -> Option<AnimationValue> {
        match keyframe_value {
            Some(keyframe_value) => {
                match composite_op {
                    CompositeOperation::Add => {
                        debug_assert!(need_underlying_value,
                                      "Should have detected we need an underlying value");
                        underlying_value.as_ref().unwrap().add(keyframe_value).ok()
                    },
                    CompositeOperation::Accumulate => {
                        debug_assert!(need_underlying_value,
                                      "Should have detected we need an underlying value");
                        underlying_value.as_ref().unwrap().accumulate(keyframe_value, 1).ok()
                    },
                    _ => None,
                }
            },
            None => {
                debug_assert!(need_underlying_value,
                              "Should have detected we need an underlying value");
                underlying_value.clone()
            },
        }
    };
    let mut composited_from_value = composite_endpoint(keyframe_from_value, segment.mFromComposite);
    let mut composited_to_value = composite_endpoint(keyframe_to_value, segment.mToComposite);

    debug_assert!(keyframe_from_value.is_some() || composited_from_value.is_some(),
                  "Should have a suitable from value to use");
    debug_assert!(keyframe_to_value.is_some() || composited_to_value.is_some(),
                  "Should have a suitable to value to use");

    // Apply iteration composite behavior.
    if iteration_composite == IterationCompositeOperation::Accumulate &&
       computed_timing.mCurrentIteration > 0 {
        let raw_last_value;
        let last_value = if !last_segment.mToValue.mServo.mRawPtr.is_null() {
            raw_last_value = unsafe { &*last_segment.mToValue.mServo.mRawPtr };
            &*AnimationValue::as_arc(&raw_last_value)
        } else {
            debug_assert!(need_underlying_value,
                          "Should have detected we need an underlying value");
            underlying_value.as_ref().unwrap()
        };

        // As with composite_endpoint, a return value of None means, "Use keyframe_value as-is."
        let apply_iteration_composite = |keyframe_value: Option<&RawOffsetArc<AnimationValue>>,
                                         composited_value: Option<AnimationValue>|
                                        -> Option<AnimationValue> {
            let count = computed_timing.mCurrentIteration;
            match composited_value {
                Some(endpoint) => last_value.accumulate(&endpoint, count)
                                            .ok()
                                            .or(Some(endpoint)),
                None => last_value.accumulate(keyframe_value.unwrap(), count)
                                  .ok(),
            }
        };

        composited_from_value = apply_iteration_composite(keyframe_from_value,
                                                          composited_from_value);
        composited_to_value = apply_iteration_composite(keyframe_to_value,
                                                        composited_to_value);
    }

    // Use the composited value if there is one, otherwise, use the original keyframe value.
    let from_value = composited_from_value.as_ref().unwrap_or_else(|| keyframe_from_value.unwrap());
    let to_value   = composited_to_value.as_ref().unwrap_or_else(|| keyframe_to_value.unwrap());

    let progress = unsafe { Gecko_GetProgressFromComputedTiming(computed_timing) };
    if segment.mToKey == segment.mFromKey {
        if progress < 0. {
            value_map.insert(property, from_value.clone());
        } else {
            value_map.insert(property, to_value.clone());
        }
        return;
    }

    let position = unsafe {
        Gecko_GetPositionInSegment(segment, progress, computed_timing.mBeforeFlag)
    };
    if let Ok(value) = from_value.interpolate(to_value, position) {
        value_map.insert(property, value);
    } else if position < 0.5 {
        value_map.insert(property, from_value.clone());
    } else {
        value_map.insert(property, to_value.clone());
    }
}

macro_rules! get_property_id_from_nscsspropertyid {
    ($property_id: ident, $ret: expr) => {{
        match PropertyId::from_nscsspropertyid($property_id) {
            Ok(property_id) => property_id,
            Err(()) => { return $ret; }
        }
    }}
}

#[no_mangle]
pub extern "C" fn Servo_AnimationValue_Serialize(value: RawServoAnimationValueBorrowed,
                                                 property: nsCSSPropertyID,
                                                 buffer: *mut nsAString)
{
    let uncomputed_value = AnimationValue::as_arc(&value).uncompute();
    let mut string = String::new();
    let rv = PropertyDeclarationBlock::with_one(uncomputed_value, Importance::Normal)
        .single_value_to_css(&get_property_id_from_nscsspropertyid!(property, ()), &mut string);
    debug_assert!(rv.is_ok());

    let buffer = unsafe { buffer.as_mut().unwrap() };
    buffer.assign_utf8(&string);
}

#[no_mangle]
pub extern "C" fn Servo_Shorthand_AnimationValues_Serialize(shorthand_property: nsCSSPropertyID,
                                                            values: RawGeckoServoAnimationValueListBorrowed,
                                                            buffer: *mut nsAString)
{
    let property_id = get_property_id_from_nscsspropertyid!(shorthand_property, ());
    let shorthand = match property_id.as_shorthand() {
        Ok(shorthand) => shorthand,
        _ => return,
    };

    // Convert RawServoAnimationValue(s) into a vector of PropertyDeclaration
    // so that we can use reference of the PropertyDeclaration without worrying
    // about its lifetime. (longhands_to_css() expects &PropertyDeclaration
    // iterator.)
    let declarations: Vec<PropertyDeclaration> =
        values.iter().map(|v| AnimationValue::as_arc(unsafe { &&*v.mRawPtr }).uncompute()).collect();

    let mut string = String::new();
    let rv = shorthand.longhands_to_css(declarations.iter(), &mut string);
    if rv.is_ok() {
        let buffer = unsafe { buffer.as_mut().unwrap() };
        buffer.assign_utf8(&string);
    }
}

#[no_mangle]
pub extern "C" fn Servo_AnimationValue_GetOpacity(value: RawServoAnimationValueBorrowed)
     -> f32
{
    let value = AnimationValue::as_arc(&value);
    if let AnimationValue::Opacity(opacity) = **value {
        opacity
    } else {
        panic!("The AnimationValue should be Opacity");
    }
}

#[no_mangle]
pub extern "C" fn Servo_AnimationValue_GetTransform(value: RawServoAnimationValueBorrowed,
                                                    list: *mut structs::RefPtr<nsCSSValueSharedList>)
{
    let value = AnimationValue::as_arc(&value);
    if let AnimationValue::Transform(ref servo_list) = **value {
        let list = unsafe { &mut *list };
        match servo_list.0 {
            Some(ref servo_list) => {
                style_structs::Box::convert_transform(servo_list, list);
            },
            None => unsafe {
                list.set_move(RefPtr::from_addrefed(Gecko_NewNoneTransform()));
            }
        }
    } else {
        panic!("The AnimationValue should be transform");
    }
}

#[no_mangle]
pub extern "C" fn Servo_AnimationValue_DeepEqual(this: RawServoAnimationValueBorrowed,
                                                 other: RawServoAnimationValueBorrowed)
     -> bool
{
    let this_value = AnimationValue::as_arc(&this);
    let other_value = AnimationValue::as_arc(&other);
    this_value == other_value
}

#[no_mangle]
pub extern "C" fn Servo_AnimationValue_Uncompute(value: RawServoAnimationValueBorrowed)
                                                 -> RawServoDeclarationBlockStrong {
    let value = AnimationValue::as_arc(&value);
    let global_style_data = &*GLOBAL_STYLE_DATA;
    Arc::new(global_style_data.shared_lock.wrap(
        PropertyDeclarationBlock::with_one(value.uncompute(), Importance::Normal))).into_strong()
}

#[no_mangle]
pub extern "C" fn Servo_StyleSet_GetBaseComputedValuesForElement(raw_data: RawServoStyleSetBorrowed,
                                                                 element: RawGeckoElementBorrowed,
                                                                 computed_values: ServoStyleContextBorrowed,
                                                                 snapshots: *const ServoElementSnapshotTable,
                                                                 pseudo_type: CSSPseudoElementType)
                                                                 -> ServoStyleContextStrong
{
    use style::style_resolver::StyleResolverForElement;

    debug_assert!(!snapshots.is_null());
    let computed_values = unsafe { ArcBorrow::from_ref(computed_values) };

    let rules = match computed_values.rules {
        None => return computed_values.clone_arc().into(),
        Some(ref rules) => rules,
    };

    let doc_data = PerDocumentStyleData::from_ffi(raw_data).borrow();
    let without_animations =
        doc_data.stylist.rule_tree().remove_animation_rules(rules);

    if without_animations == *rules {
        return computed_values.clone_arc().into();
    }

    let element = GeckoElement(element);

    let element_data = match element.borrow_data() {
        Some(data) => data,
        None => return computed_values.clone_arc().into(),
    };
    let styles = &element_data.styles;

    if let Some(pseudo) = PseudoElement::from_pseudo_type(pseudo_type) {
        // This style already doesn't have animations.
        return styles
            .pseudos
            .get(&pseudo)
            .expect("GetBaseComputedValuesForElement for an unexisting pseudo?")
            .clone().into();
    }

    let global_style_data = &*GLOBAL_STYLE_DATA;
    let guard = global_style_data.shared_lock.read();
    let shared = create_shared_context(&global_style_data,
                                       &guard,
                                       &doc_data,
                                       TraversalFlags::empty(),
                                       unsafe { &*snapshots });
    let mut tlc = ThreadLocalStyleContext::new(&shared);
    let mut context = StyleContext {
        shared: &shared,
        thread_local: &mut tlc,
    };

    // This currently ignores visited styles, which seems acceptable, as
    // existing browsers don't appear to animate visited styles.
    let inputs =
        CascadeInputs {
            rules: Some(without_animations),
            visited_rules: None,
        };

    StyleResolverForElement::new(element, &mut context, RuleInclusion::All)
        .cascade_style_and_visited_with_default_parents(inputs)
        .into()
}

#[no_mangle]
pub extern "C" fn Servo_ComputedValues_ExtractAnimationValue(computed_values: ServoStyleContextBorrowed,
                                                             property_id: nsCSSPropertyID)
                                                             -> RawServoAnimationValueStrong
{
    let property = match AnimatableLonghand::from_nscsspropertyid(property_id) {
        Some(longhand) => longhand,
        None => return Strong::null(),
    };

    Arc::new(AnimationValue::from_computed_values(&property, &computed_values)).into_strong()
}

#[no_mangle]
pub extern "C" fn Servo_Property_IsAnimatable(property: nsCSSPropertyID) -> bool {
    use style::properties::animated_properties;
    animated_properties::nscsspropertyid_is_animatable(property)
}

#[no_mangle]
pub extern "C" fn Servo_Property_IsTransitionable(property: nsCSSPropertyID) -> bool {
    use style::properties::animated_properties;
    animated_properties::nscsspropertyid_is_transitionable(property)
}

#[no_mangle]
pub extern "C" fn Servo_Property_IsDiscreteAnimatable(property: nsCSSPropertyID) -> bool {
    match AnimatableLonghand::from_nscsspropertyid(property) {
        Some(longhand) => longhand.is_discrete(),
        None => false
    }
}

#[no_mangle]
pub extern "C" fn Servo_StyleWorkerThreadCount() -> u32 {
    STYLE_THREAD_POOL.num_threads as u32
}

#[no_mangle]
pub extern "C" fn Servo_Element_ClearData(element: RawGeckoElementBorrowed) {
    unsafe { GeckoElement(element).clear_data() };
}

#[no_mangle]
pub extern "C" fn Servo_Element_SizeOfExcludingThis(malloc_size_of: MallocSizeOf,
                                                    seen_ptrs: *mut SeenPtrs,
                                                    element: RawGeckoElementBorrowed) -> usize {
    let malloc_size_of = malloc_size_of.unwrap();
    let element = GeckoElement(element);
    let borrow = element.borrow_data();
    if let Some(data) = borrow {
        let mut state = SizeOfState {
            malloc_size_of: malloc_size_of,
            seen_ptrs: seen_ptrs,
        };
        (*data).malloc_size_of_children(&mut state)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn Servo_StyleSheet_Empty(mode: SheetParsingMode) -> RawServoStyleSheetContentsStrong {
    let global_style_data = &*GLOBAL_STYLE_DATA;
    let origin = match mode {
        SheetParsingMode::eAuthorSheetFeatures => Origin::Author,
        SheetParsingMode::eUserSheetFeatures => Origin::User,
        SheetParsingMode::eAgentSheetFeatures => Origin::UserAgent,
        SheetParsingMode::eSafeAgentSheetFeatures => Origin::UserAgent,
    };
    let shared_lock = &global_style_data.shared_lock;
    Arc::new(
        StylesheetContents::from_str(
            "",
            unsafe { dummy_url_data() }.clone(),
            origin,
            shared_lock,
            /* loader = */ None,
            &NullReporter,
            QuirksMode::NoQuirks,
            0
        )
    ).into_strong()
}

#[no_mangle]
pub extern "C" fn Servo_StyleSheet_FromUTF8Bytes(
    loader: *mut Loader,
    stylesheet: *mut ServoStyleSheet,
    data: *const nsACString,
    mode: SheetParsingMode,
    extra_data: *mut URLExtraData,
    line_number_offset: u32,
    quirks_mode: nsCompatibility
) -> RawServoStyleSheetContentsStrong {
    let global_style_data = &*GLOBAL_STYLE_DATA;
    let input = unsafe { data.as_ref().unwrap().as_str_unchecked() };

    let origin = match mode {
        SheetParsingMode::eAuthorSheetFeatures => Origin::Author,
        SheetParsingMode::eUserSheetFeatures => Origin::User,
        SheetParsingMode::eAgentSheetFeatures => Origin::UserAgent,
        SheetParsingMode::eSafeAgentSheetFeatures => Origin::UserAgent,
    };

    let reporter = ErrorReporter::new(stylesheet, loader, extra_data);
    let url_data = unsafe { RefPtr::from_ptr_ref(&extra_data) };
    let loader = if loader.is_null() {
        None
    } else {
        Some(StylesheetLoader::new(loader, stylesheet, ptr::null_mut()))
    };

    // FIXME(emilio): loader.as_ref() doesn't typecheck for some reason?
    let loader: Option<&StyleStylesheetLoader> = match loader {
        None => None,
        Some(ref s) => Some(s),
    };


    Arc::new(StylesheetContents::from_str(
        input, url_data.clone(), origin,
        &global_style_data.shared_lock, loader, &reporter,
        quirks_mode.into(), line_number_offset as u64)
    ).into_strong()
}

#[no_mangle]
pub extern "C" fn Servo_StyleSet_AppendStyleSheet(
    raw_data: RawServoStyleSetBorrowed,
    sheet: *const ServoStyleSheet,
) {
    let global_style_data = &*GLOBAL_STYLE_DATA;
    let mut data = PerDocumentStyleData::from_ffi(raw_data).borrow_mut();
    let mut data = &mut *data;
    let guard = global_style_data.shared_lock.read();
    data.stylesheets.append_stylesheet(
        &data.stylist,
        unsafe { GeckoStyleSheet::new(sheet) },
        &guard
    );
    data.clear_stylist();
}

#[no_mangle]
pub extern "C" fn Servo_StyleSet_MediumFeaturesChanged(
    raw_data: RawServoStyleSetBorrowed,
    viewport_units_used: *mut bool,
) -> bool {
    let global_style_data = &*GLOBAL_STYLE_DATA;
    let guard = global_style_data.shared_lock.read();

    // NOTE(emilio): We don't actually need to flush the stylist here and ensure
    // it's up to date.
    //
    // In case it isn't we would trigger a rebuild + restyle as needed too.
    //
    // We need to ensure the default computed values are up to date though,
    // because those can influence the result of media query evaluation.
    //
    // FIXME(emilio, bug 1369984): do the computation conditionally, to do it
    // less often.
    let mut data = PerDocumentStyleData::from_ffi(raw_data).borrow_mut();

    unsafe {
        *viewport_units_used = data.stylist.device().used_viewport_size();
    }
    data.stylist.device_mut().reset_computed_values();
    let rules_changed = data.stylist.media_features_change_changed_style(
        data.stylesheets.iter(),
        &guard,
    );

    rules_changed
}

#[no_mangle]
pub extern "C" fn Servo_StyleSet_PrependStyleSheet(
    raw_data: RawServoStyleSetBorrowed,
    sheet: *const ServoStyleSheet,
) {
    let global_style_data = &*GLOBAL_STYLE_DATA;
    let mut data = PerDocumentStyleData::from_ffi(raw_data).borrow_mut();
    let mut data = &mut *data;
    let guard = global_style_data.shared_lock.read();
    data.stylesheets.prepend_stylesheet(
        &data.stylist,
        unsafe { GeckoStyleSheet::new(sheet) },
        &guard,
    );
    data.clear_stylist();
}

#[no_mangle]
pub extern "C" fn Servo_StyleSet_InsertStyleSheetBefore(
    raw_data: RawServoStyleSetBorrowed,
    sheet: *const ServoStyleSheet,
    before_sheet: *const ServoStyleSheet
) {
    let global_style_data = &*GLOBAL_STYLE_DATA;
    let mut data = PerDocumentStyleData::from_ffi(raw_data).borrow_mut();
    let mut data = &mut *data;
    let guard = global_style_data.shared_lock.read();
    data.stylesheets.insert_stylesheet_before(
        &data.stylist,
        unsafe { GeckoStyleSheet::new(sheet) },
        unsafe { GeckoStyleSheet::new(before_sheet) },
        &guard,
    );
    data.clear_stylist();
}

#[no_mangle]
pub extern "C" fn Servo_StyleSet_RemoveStyleSheet(
    raw_data: RawServoStyleSetBorrowed,
    sheet: *const ServoStyleSheet
) {
    let global_style_data = &*GLOBAL_STYLE_DATA;
    let mut data = PerDocumentStyleData::from_ffi(raw_data).borrow_mut();
    let mut data = &mut *data;
    let guard = global_style_data.shared_lock.read();
    data.stylesheets.remove_stylesheet(
        &data.stylist,
        unsafe { GeckoStyleSheet::new(sheet) },
        &guard,
    );
    data.clear_stylist();
}

#[no_mangle]
pub extern "C" fn Servo_StyleSet_FlushStyleSheets(
    raw_data: RawServoStyleSetBorrowed,
    doc_element: RawGeckoElementBorrowedOrNull,
) {
    let global_style_data = &*GLOBAL_STYLE_DATA;
    let guard = global_style_data.shared_lock.read();
    let mut data = PerDocumentStyleData::from_ffi(raw_data).borrow_mut();
    let doc_element = doc_element.map(GeckoElement);
    data.flush_stylesheets(&guard, doc_element);
}

#[no_mangle]
pub extern "C" fn Servo_StyleSet_NoteStyleSheetsChanged(
    raw_data: RawServoStyleSetBorrowed,
    author_style_disabled: bool,
) {
    let mut data = PerDocumentStyleData::from_ffi(raw_data).borrow_mut();
    data.stylesheets.force_dirty();
    data.stylesheets.set_author_style_disabled(author_style_disabled);
    data.clear_stylist();
}

#[no_mangle]
pub extern "C" fn Servo_StyleSheet_HasRules(
    raw_contents: RawServoStyleSheetContentsBorrowed
) -> bool {
    let global_style_data = &*GLOBAL_STYLE_DATA;
    let guard = global_style_data.shared_lock.read();
    !StylesheetContents::as_arc(&raw_contents)
        .rules.read_with(&guard).0.is_empty()
}

#[no_mangle]
pub extern "C" fn Servo_StyleSheet_GetRules(
    sheet: RawServoStyleSheetContentsBorrowed
) -> ServoCssRulesStrong {
    StylesheetContents::as_arc(&sheet).rules.clone().into_strong()
}

#[no_mangle]
pub extern "C" fn Servo_StyleSheet_Clone(
    raw_sheet: RawServoStyleSheetContentsBorrowed,
    reference_sheet: *const ServoStyleSheet,
) -> RawServoStyleSheetContentsStrong {
    use style::shared_lock::{DeepCloneParams, DeepCloneWithLock};
    let global_style_data = &*GLOBAL_STYLE_DATA;
    let guard = global_style_data.shared_lock.read();
    let contents = StylesheetContents::as_arc(&raw_sheet);
    let params = DeepCloneParams { reference_sheet };

    Arc::new(contents.deep_clone_with_lock(
        &global_style_data.shared_lock,
        &guard,
        &params,
    )).into_strong()
}

#[no_mangle]
pub extern "C" fn Servo_StyleSheet_SizeOfIncludingThis(
    malloc_size_of: MallocSizeOf,
    sheet: RawServoStyleSheetContentsBorrowed
) -> usize {
    let global_style_data = &*GLOBAL_STYLE_DATA;
    let guard = global_style_data.shared_lock.read();
    let malloc_size_of = malloc_size_of.unwrap();
    StylesheetContents::as_arc(&sheet)
        .malloc_size_of_children(&guard, malloc_size_of)
}

fn read_locked_arc<T, R, F>(raw: &<Locked<T> as HasFFI>::FFIType, func: F) -> R
    where Locked<T>: HasArcFFI, F: FnOnce(&T) -> R
{
    let global_style_data = &*GLOBAL_STYLE_DATA;
    let guard = global_style_data.shared_lock.read();
    func(Locked::<T>::as_arc(&raw).read_with(&guard))
}

fn write_locked_arc<T, R, F>(raw: &<Locked<T> as HasFFI>::FFIType, func: F) -> R
    where Locked<T>: HasArcFFI, F: FnOnce(&mut T) -> R
{
    let global_style_data = &*GLOBAL_STYLE_DATA;
    let mut guard = global_style_data.shared_lock.write();
    func(Locked::<T>::as_arc(&raw).write_with(&mut guard))
}

#[no_mangle]
pub extern "C" fn Servo_CssRules_ListTypes(rules: ServoCssRulesBorrowed,
                                           result: nsTArrayBorrowed_uintptr_t) {
    read_locked_arc(rules, |rules: &CssRules| {
        let iter = rules.0.iter().map(|rule| rule.rule_type() as usize);
        let (size, upper) = iter.size_hint();
        debug_assert_eq!(size, upper.unwrap());
        unsafe { result.set_len(size as u32) };
        result.iter_mut().zip(iter).fold((), |_, (r, v)| *r = v);
    })
}

#[no_mangle]
pub extern "C" fn Servo_CssRules_InsertRule(
    rules: ServoCssRulesBorrowed,
    contents: RawServoStyleSheetContentsBorrowed,
    rule: *const nsACString,
    index: u32,
    nested: bool,
    loader: *mut Loader,
    gecko_stylesheet: *mut ServoStyleSheet,
    rule_type: *mut u16,
) -> nsresult {
    let loader = if loader.is_null() {
        None
    } else {
        Some(StylesheetLoader::new(loader, gecko_stylesheet, ptr::null_mut()))
    };
    let loader = loader.as_ref().map(|loader| loader as &StyleStylesheetLoader);
    let rule = unsafe { rule.as_ref().unwrap().as_str_unchecked() };

    let global_style_data = &*GLOBAL_STYLE_DATA;
    let contents = StylesheetContents::as_arc(&contents);
    let result = Locked::<CssRules>::as_arc(&rules).insert_rule(
        &global_style_data.shared_lock,
        rule,
        contents,
        index as usize,
        nested,
        loader
    );

    match result {
        Ok(new_rule) => {
            *unsafe { rule_type.as_mut().unwrap() } = new_rule.rule_type() as u16;
            nsresult::NS_OK
        }
        Err(err) => err.into(),
    }
}

#[no_mangle]
pub extern "C" fn Servo_CssRules_DeleteRule(
    rules: ServoCssRulesBorrowed,
    index: u32
) -> nsresult {
    write_locked_arc(rules, |rules: &mut CssRules| {
        match rules.remove_rule(index as usize) {
            Ok(_) => nsresult::NS_OK,
            Err(err) => err.into()
        }
    })
}

macro_rules! impl_basic_rule_funcs_without_getter {
    { ($rule_type:ty, $raw_type:ty),
        debug: $debug:ident,
        to_css: $to_css:ident,
    } => {
        #[no_mangle]
        pub extern "C" fn $debug(rule: &$raw_type, result: *mut nsACString) {
            read_locked_arc(rule, |rule: &$rule_type| {
                write!(unsafe { result.as_mut().unwrap() }, "{:?}", *rule).unwrap();
            })
        }

        #[no_mangle]
        pub extern "C" fn $to_css(rule: &$raw_type, result: *mut nsAString) {
            let global_style_data = &*GLOBAL_STYLE_DATA;
            let guard = global_style_data.shared_lock.read();
            let rule = Locked::<$rule_type>::as_arc(&rule);
            rule.read_with(&guard).to_css(&guard, unsafe { result.as_mut().unwrap() }).unwrap();
        }
    }
}

macro_rules! impl_basic_rule_funcs {
    { ($name:ident, $rule_type:ty, $raw_type:ty),
        getter: $getter:ident,
        debug: $debug:ident,
        to_css: $to_css:ident,
    } => {
        #[no_mangle]
        pub extern "C" fn $getter(rules: ServoCssRulesBorrowed, index: u32,
                                  line: *mut u32, column: *mut u32)
            -> Strong<$raw_type> {
            let global_style_data = &*GLOBAL_STYLE_DATA;
            let guard = global_style_data.shared_lock.read();
            let rules = Locked::<CssRules>::as_arc(&rules).read_with(&guard);
            let index = index as usize;

            if index >= rules.0.len() {
                return Strong::null();
            }

            match rules.0[index] {
                CssRule::$name(ref rule) => {
                    let location = rule.read_with(&guard).source_location;
                    *unsafe { line.as_mut().unwrap() } = location.line as u32;
                    *unsafe { column.as_mut().unwrap() } = location.column as u32;
                    rule.clone().into_strong()
                },
                _ => {
                    Strong::null()
                }
            }
        }

        impl_basic_rule_funcs_without_getter! { ($rule_type, $raw_type),
            debug: $debug,
            to_css: $to_css,
        }
    }
}

macro_rules! impl_group_rule_funcs {
    { ($name:ident, $rule_type:ty, $raw_type:ty),
      get_rules: $get_rules:ident,
      $($basic:tt)+
    } => {
        impl_basic_rule_funcs! { ($name, $rule_type, $raw_type), $($basic)+ }

        #[no_mangle]
        pub extern "C" fn $get_rules(rule: &$raw_type) -> ServoCssRulesStrong {
            read_locked_arc(rule, |rule: &$rule_type| {
                rule.rules.clone().into_strong()
            })
        }
    }
}

impl_basic_rule_funcs! { (Style, StyleRule, RawServoStyleRule),
    getter: Servo_CssRules_GetStyleRuleAt,
    debug: Servo_StyleRule_Debug,
    to_css: Servo_StyleRule_GetCssText,
}

impl_basic_rule_funcs! { (Import, ImportRule, RawServoImportRule),
    getter: Servo_CssRules_GetImportRuleAt,
    debug: Servo_ImportRule_Debug,
    to_css: Servo_ImportRule_GetCssText,
}

impl_basic_rule_funcs_without_getter! { (Keyframe, RawServoKeyframe),
    debug: Servo_Keyframe_Debug,
    to_css: Servo_Keyframe_GetCssText,
}

impl_basic_rule_funcs! { (Keyframes, KeyframesRule, RawServoKeyframesRule),
    getter: Servo_CssRules_GetKeyframesRuleAt,
    debug: Servo_KeyframesRule_Debug,
    to_css: Servo_KeyframesRule_GetCssText,
}

impl_group_rule_funcs! { (Media, MediaRule, RawServoMediaRule),
    get_rules: Servo_MediaRule_GetRules,
    getter: Servo_CssRules_GetMediaRuleAt,
    debug: Servo_MediaRule_Debug,
    to_css: Servo_MediaRule_GetCssText,
}

impl_basic_rule_funcs! { (Namespace, NamespaceRule, RawServoNamespaceRule),
    getter: Servo_CssRules_GetNamespaceRuleAt,
    debug: Servo_NamespaceRule_Debug,
    to_css: Servo_NamespaceRule_GetCssText,
}

impl_basic_rule_funcs! { (Page, PageRule, RawServoPageRule),
    getter: Servo_CssRules_GetPageRuleAt,
    debug: Servo_PageRule_Debug,
    to_css: Servo_PageRule_GetCssText,
}

impl_group_rule_funcs! { (Supports, SupportsRule, RawServoSupportsRule),
    get_rules: Servo_SupportsRule_GetRules,
    getter: Servo_CssRules_GetSupportsRuleAt,
    debug: Servo_SupportsRule_Debug,
    to_css: Servo_SupportsRule_GetCssText,
}

impl_group_rule_funcs! { (Document, DocumentRule, RawServoDocumentRule),
    get_rules: Servo_DocumentRule_GetRules,
    getter: Servo_CssRules_GetDocumentRuleAt,
    debug: Servo_DocumentRule_Debug,
    to_css: Servo_DocumentRule_GetCssText,
}

impl_basic_rule_funcs! { (FontFeatureValues, FontFeatureValuesRule, RawServoFontFeatureValuesRule),
    getter: Servo_CssRules_GetFontFeatureValuesRuleAt,
    debug: Servo_FontFeatureValuesRule_Debug,
    to_css: Servo_FontFeatureValuesRule_GetCssText,
}

macro_rules! impl_getter_for_embedded_rule {
    ($getter:ident: $name:ident -> $ty:ty) => {
        #[no_mangle]
        pub extern "C" fn $getter(rules: ServoCssRulesBorrowed, index: u32) -> *mut $ty
        {
            let global_style_data = &*GLOBAL_STYLE_DATA;
            let guard = global_style_data.shared_lock.read();
            let rules = Locked::<CssRules>::as_arc(&rules).read_with(&guard);
            match rules.0[index as usize] {
                CssRule::$name(ref rule) => rule.read_with(&guard).get(),
                _ => unreachable!(concat!(stringify!($getter), " should only be called on a ",
                                          stringify!($name), " rule")),
            }
        }
    }
}

impl_getter_for_embedded_rule!(Servo_CssRules_GetFontFaceRuleAt:
                              FontFace -> nsCSSFontFaceRule);
impl_getter_for_embedded_rule!(Servo_CssRules_GetCounterStyleRuleAt:
                              CounterStyle -> nsCSSCounterStyleRule);

#[no_mangle]
pub extern "C" fn Servo_StyleRule_GetStyle(rule: RawServoStyleRuleBorrowed) -> RawServoDeclarationBlockStrong {
    read_locked_arc(rule, |rule: &StyleRule| {
        rule.block.clone().into_strong()
    })
}

#[no_mangle]
pub extern "C" fn Servo_StyleRule_SetStyle(rule: RawServoStyleRuleBorrowed,
                                           declarations: RawServoDeclarationBlockBorrowed) {
    let declarations = Locked::<PropertyDeclarationBlock>::as_arc(&declarations);
    write_locked_arc(rule, |rule: &mut StyleRule| {
        rule.block = declarations.clone_arc();
    })
}

#[no_mangle]
pub extern "C" fn Servo_StyleRule_GetSelectorText(rule: RawServoStyleRuleBorrowed, result: *mut nsAString) {
    read_locked_arc(rule, |rule: &StyleRule| {
        rule.selectors.to_css(unsafe { result.as_mut().unwrap() }).unwrap();
    })
}

#[no_mangle]
pub extern "C" fn Servo_StyleRule_GetSelectorTextAtIndex(rule: RawServoStyleRuleBorrowed,
                                                         index: u32,
                                                         result: *mut nsAString) {
    read_locked_arc(rule, |rule: &StyleRule| {
        let index = index as usize;
        if index >= rule.selectors.0.len() {
            return;
        }
        rule.selectors.0[index].to_css(unsafe { result.as_mut().unwrap() }).unwrap();
    })
}

#[no_mangle]
pub extern "C" fn Servo_StyleRule_GetSelectorCount(rule: RawServoStyleRuleBorrowed, count: *mut u32) {
    read_locked_arc(rule, |rule: &StyleRule| {
        *unsafe { count.as_mut().unwrap() } = rule.selectors.0.len() as u32;
    })
}

#[no_mangle]
pub extern "C" fn Servo_StyleRule_GetSpecificityAtIndex(
    rule: RawServoStyleRuleBorrowed,
    index: u32,
    specificity: *mut u64
) {
    read_locked_arc(rule, |rule: &StyleRule| {
        let mut specificity =  unsafe { specificity.as_mut().unwrap() };
        let index = index as usize;
        if index >= rule.selectors.0.len() {
            *specificity = 0;
            return;
        }
        *specificity = rule.selectors.0[index].specificity() as u64;
    })
}

#[no_mangle]
pub extern "C" fn Servo_StyleRule_SelectorMatchesElement(rule: RawServoStyleRuleBorrowed,
                                                         element: RawGeckoElementBorrowed,
                                                         index: u32,
                                                         pseudo_type: CSSPseudoElementType) -> bool {
    read_locked_arc(rule, |rule: &StyleRule| {
        let index = index as usize;
        if index >= rule.selectors.0.len() {
            return false;
        }

        let selector = &rule.selectors.0[index];
        let mut matching_mode = MatchingMode::Normal;

        match PseudoElement::from_pseudo_type(pseudo_type) {
            Some(pseudo) => {
                // We need to make sure that the requested pseudo element type
                // matches the selector pseudo element type before proceeding.
                match selector.pseudo_element() {
                    Some(selector_pseudo) if *selector_pseudo == pseudo => {
                        matching_mode = MatchingMode::ForStatelessPseudoElement
                    },
                    _ => return false,
                };
            },
            None => {
                // Do not attempt to match if a pseudo element is requested and
                // this is not a pseudo element selector, or vice versa.
                if selector.has_pseudo_element() {
                    return false;
                }
            },
        };

        let element = GeckoElement(element);
        let mut ctx = MatchingContext::new(matching_mode, None, element.owner_document_quirks_mode());
        matches_selector(selector, 0, None, &element, &mut ctx, &mut |_, _| {})
    })
}

#[no_mangle]
pub extern "C" fn Servo_ImportRule_GetHref(rule: RawServoImportRuleBorrowed, result: *mut nsAString) {
    read_locked_arc(rule, |rule: &ImportRule| {
        write!(unsafe { &mut *result }, "{}", rule.url.as_str()).unwrap();
    })
}

#[no_mangle]
pub extern "C" fn Servo_ImportRule_GetSheet(
    rule: RawServoImportRuleBorrowed
) -> *const ServoStyleSheet {
    read_locked_arc(rule, |rule: &ImportRule| {
        rule.stylesheet.0.raw() as *const ServoStyleSheet
    })
}

#[no_mangle]
pub extern "C" fn Servo_Keyframe_GetKeyText(
    keyframe: RawServoKeyframeBorrowed,
    result: *mut nsAString
) {
    read_locked_arc(keyframe, |keyframe: &Keyframe| {
        keyframe.selector.to_css(unsafe { result.as_mut().unwrap() }).unwrap()
    })
}

#[no_mangle]
pub extern "C" fn Servo_Keyframe_SetKeyText(keyframe: RawServoKeyframeBorrowed, text: *const nsACString) -> bool {
    let text = unsafe { text.as_ref().unwrap().as_str_unchecked() };
    let mut input = ParserInput::new(&text);
    if let Ok(selector) = Parser::new(&mut input).parse_entirely(KeyframeSelector::parse) {
        write_locked_arc(keyframe, |keyframe: &mut Keyframe| {
            keyframe.selector = selector;
        });
        true
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn Servo_Keyframe_GetStyle(keyframe: RawServoKeyframeBorrowed) -> RawServoDeclarationBlockStrong {
    read_locked_arc(keyframe, |keyframe: &Keyframe| keyframe.block.clone().into_strong())
}

#[no_mangle]
pub extern "C" fn Servo_Keyframe_SetStyle(keyframe: RawServoKeyframeBorrowed,
                                          declarations: RawServoDeclarationBlockBorrowed) {
    let declarations = Locked::<PropertyDeclarationBlock>::as_arc(&declarations);
    write_locked_arc(keyframe, |keyframe: &mut Keyframe| {
        keyframe.block = declarations.clone_arc();
    })
}

#[no_mangle]
pub extern "C" fn Servo_KeyframesRule_GetName(rule: RawServoKeyframesRuleBorrowed) -> *mut nsIAtom {
    read_locked_arc(rule, |rule: &KeyframesRule| rule.name.as_atom().as_ptr())
}

#[no_mangle]
pub extern "C" fn Servo_KeyframesRule_SetName(rule: RawServoKeyframesRuleBorrowed, name: *mut nsIAtom) {
    write_locked_arc(rule, |rule: &mut KeyframesRule| {
        rule.name = KeyframesName::Ident(CustomIdent(unsafe { Atom::from_addrefed(name) }));
    })
}

#[no_mangle]
pub extern "C" fn Servo_KeyframesRule_GetCount(rule: RawServoKeyframesRuleBorrowed) -> u32 {
    read_locked_arc(rule, |rule: &KeyframesRule| rule.keyframes.len() as u32)
}

#[no_mangle]
pub extern "C" fn Servo_KeyframesRule_GetKeyframe(rule: RawServoKeyframesRuleBorrowed, index: u32)
                                                  -> RawServoKeyframeStrong {
    read_locked_arc(rule, |rule: &KeyframesRule| {
        rule.keyframes[index as usize].clone().into_strong()
    })
}

#[no_mangle]
pub extern "C" fn Servo_KeyframesRule_FindRule(rule: RawServoKeyframesRuleBorrowed,
                                               key: *const nsACString) -> u32 {
    let key = unsafe { key.as_ref().unwrap().as_str_unchecked() };
    let global_style_data = &*GLOBAL_STYLE_DATA;
    let guard = global_style_data.shared_lock.read();
    Locked::<KeyframesRule>::as_arc(&rule).read_with(&guard)
        .find_rule(&guard, key).map(|index| index as u32)
        .unwrap_or(u32::max_value())
}

#[no_mangle]
pub extern "C" fn Servo_KeyframesRule_AppendRule(
    rule: RawServoKeyframesRuleBorrowed,
    contents: RawServoStyleSheetContentsBorrowed,
    css: *const nsACString
) -> bool {
    let css = unsafe { css.as_ref().unwrap().as_str_unchecked() };
    let contents = StylesheetContents::as_arc(&contents);
    let global_style_data = &*GLOBAL_STYLE_DATA;
    match Keyframe::parse(css, &contents, &global_style_data.shared_lock) {
        Ok(keyframe) => {
            write_locked_arc(rule, |rule: &mut KeyframesRule| {
                rule.keyframes.push(keyframe);
            });
            true
        }
        Err(..) => false,
    }
}

#[no_mangle]
pub extern "C" fn Servo_KeyframesRule_DeleteRule(rule: RawServoKeyframesRuleBorrowed, index: u32) {
    write_locked_arc(rule, |rule: &mut KeyframesRule| {
        rule.keyframes.remove(index as usize);
    })
}

#[no_mangle]
pub extern "C" fn Servo_MediaRule_GetMedia(rule: RawServoMediaRuleBorrowed) -> RawServoMediaListStrong {
    read_locked_arc(rule, |rule: &MediaRule| {
        rule.media_queries.clone().into_strong()
    })
}

#[no_mangle]
pub extern "C" fn Servo_NamespaceRule_GetPrefix(rule: RawServoNamespaceRuleBorrowed) -> *mut nsIAtom {
    read_locked_arc(rule, |rule: &NamespaceRule| {
        rule.prefix.as_ref().unwrap_or(&atom!("")).as_ptr()
    })
}

#[no_mangle]
pub extern "C" fn Servo_NamespaceRule_GetURI(rule: RawServoNamespaceRuleBorrowed) -> *mut nsIAtom {
    read_locked_arc(rule, |rule: &NamespaceRule| rule.url.0.as_ptr())
}

#[no_mangle]
pub extern "C" fn Servo_PageRule_GetStyle(rule: RawServoPageRuleBorrowed) -> RawServoDeclarationBlockStrong {
    read_locked_arc(rule, |rule: &PageRule| {
        rule.block.clone().into_strong()
    })
}

#[no_mangle]
pub extern "C" fn Servo_PageRule_SetStyle(rule: RawServoPageRuleBorrowed,
                                           declarations: RawServoDeclarationBlockBorrowed) {
    let declarations = Locked::<PropertyDeclarationBlock>::as_arc(&declarations);
    write_locked_arc(rule, |rule: &mut PageRule| {
        rule.block = declarations.clone_arc();
    })
}

#[no_mangle]
pub extern "C" fn Servo_SupportsRule_GetConditionText(rule: RawServoSupportsRuleBorrowed,
                                                      result: *mut nsAString) {
    read_locked_arc(rule, |rule: &SupportsRule| {
        rule.condition.to_css(unsafe { result.as_mut().unwrap() }).unwrap();
    })
}

#[no_mangle]
pub extern "C" fn Servo_DocumentRule_GetConditionText(rule: RawServoDocumentRuleBorrowed,
                                                      result: *mut nsAString) {
    read_locked_arc(rule, |rule: &DocumentRule| {
        rule.condition.to_css(unsafe { result.as_mut().unwrap() }).unwrap();
    })
}

#[no_mangle]
pub extern "C" fn Servo_FontFeatureValuesRule_GetFontFamily(rule: RawServoFontFeatureValuesRuleBorrowed,
                                                            result: *mut nsAString) {
    read_locked_arc(rule, |rule: &FontFeatureValuesRule| {
        rule.font_family_to_css(unsafe { result.as_mut().unwrap() }).unwrap();
    })
}

#[no_mangle]
pub extern "C" fn Servo_FontFeatureValuesRule_GetValueText(rule: RawServoFontFeatureValuesRuleBorrowed,
                                                           result: *mut nsAString) {
    read_locked_arc(rule, |rule: &FontFeatureValuesRule| {
        rule.value_to_css(unsafe { result.as_mut().unwrap() }).unwrap();
    })
}

#[no_mangle]
pub extern "C" fn Servo_ComputedValues_GetForAnonymousBox(parent_style_or_null: ServoStyleContextBorrowedOrNull,
                                                          pseudo_tag: *mut nsIAtom,
                                                          raw_data: RawServoStyleSetBorrowed)
     -> ServoStyleContextStrong {
    let global_style_data = &*GLOBAL_STYLE_DATA;
    let guard = global_style_data.shared_lock.read();
    let guards = StylesheetGuards::same(&guard);
    let data = PerDocumentStyleData::from_ffi(raw_data).borrow_mut();
    let atom = Atom::from(pseudo_tag);
    let pseudo = PseudoElement::from_anon_box_atom(&atom)
        .expect("Not an anon box pseudo?");

    let mut cascade_flags = SKIP_ROOT_AND_ITEM_BASED_DISPLAY_FIXUP;
    if pseudo.is_fieldset_content() {
        cascade_flags.insert(IS_FIELDSET_CONTENT);
    }
    let metrics = get_metrics_provider_for_product();
    data.stylist.precomputed_values_for_pseudo(
        &guards,
        &pseudo,
        parent_style_or_null.map(|x| &*x),
        cascade_flags, &metrics
    ).into()
}

#[no_mangle]
pub extern "C" fn Servo_ResolvePseudoStyle(element: RawGeckoElementBorrowed,
                                           pseudo_type: CSSPseudoElementType,
                                           is_probe: bool,
                                           inherited_style: ServoStyleContextBorrowedOrNull,
                                           raw_data: RawServoStyleSetBorrowed)
     -> ServoStyleContextStrong
{
    let element = GeckoElement(element);
    let data = unsafe { element.ensure_data() };
    let doc_data = PerDocumentStyleData::from_ffi(raw_data).borrow();

    debug!("Servo_ResolvePseudoStyle: {:?} {:?}, is_probe: {}",
           element, PseudoElement::from_pseudo_type(pseudo_type), is_probe);

    // FIXME(bholley): Assert against this.
    if !data.has_styles() {
        warn!("Calling Servo_ResolvePseudoStyle on unstyled element");
        return if is_probe {
            Strong::null()
        } else {
            doc_data.default_computed_values().clone().into()
        };
    }

    let pseudo = PseudoElement::from_pseudo_type(pseudo_type)
                    .expect("ResolvePseudoStyle with a non-pseudo?");

    let global_style_data = &*GLOBAL_STYLE_DATA;
    let guard = global_style_data.shared_lock.read();
    let style = get_pseudo_style(
        &guard,
        element,
        &pseudo,
        RuleInclusion::All,
        &data.styles,
        inherited_style,
        &*doc_data,
        is_probe,
    );

    match style {
        Some(s) => s.into(),
        None => {
            debug_assert!(is_probe);
            Strong::null()
        }
    }
}

#[no_mangle]
pub extern "C" fn Servo_SetExplicitStyle(element: RawGeckoElementBorrowed,
                                         style: ServoStyleContextBorrowed)
{
    let element = GeckoElement(element);
    debug!("Servo_SetExplicitStyle: {:?}", element);
    // We only support this API for initial styling. There's no reason it couldn't
    // work for other things, we just haven't had a reason to do so.
    debug_assert!(element.get_data().is_none());
    let mut data = unsafe { element.ensure_data() };
    data.styles.primary = Some(unsafe { ArcBorrow::from_ref(style) }.clone_arc());
}

#[no_mangle]
pub extern "C" fn Servo_HasAuthorSpecifiedRules(element: RawGeckoElementBorrowed,
                                                rule_type_mask: u32,
                                                author_colors_allowed: bool)
    -> bool
{
    let element = GeckoElement(element);

    let data =
        element.borrow_data()
        .expect("calling Servo_HasAuthorSpecifiedRules on an unstyled element");

    let primary_style = data.styles.primary();

    let guard = (*GLOBAL_STYLE_DATA).shared_lock.read();
    let guards = StylesheetGuards::same(&guard);

    primary_style.rules().has_author_specified_rules(element,
                                                     &guards,
                                                     rule_type_mask,
                                                     author_colors_allowed)
}

fn get_pseudo_style(
    guard: &SharedRwLockReadGuard,
    element: GeckoElement,
    pseudo: &PseudoElement,
    rule_inclusion: RuleInclusion,
    styles: &ElementStyles,
    inherited_styles: Option<&ComputedValues>,
    doc_data: &PerDocumentStyleDataImpl,
    is_probe: bool,
) -> Option<Arc<ComputedValues>> {
    let style = match pseudo.cascade_type() {
        PseudoElementCascadeType::Eager => {
            match *pseudo {
                PseudoElement::FirstLetter => {
                    styles.pseudos.get(&pseudo).and_then(|pseudo_styles| {
                        // inherited_styles can be None when doing lazy resolution
                        // (e.g. for computed style) or when probing.  In that case
                        // we just inherit from our element, which is what Gecko
                        // does in that situation.  What should actually happen in
                        // the computed style case is a bit unclear.
                        let inherited_styles =
                            inherited_styles.unwrap_or(styles.primary());
                        let guards = StylesheetGuards::same(guard);
                        let metrics = get_metrics_provider_for_product();
                        let inputs = CascadeInputs::new_from_style(pseudo_styles);
                        doc_data.stylist
                            .compute_pseudo_element_style_with_inputs(
                                &inputs,
                                pseudo,
                                &guards,
                                inherited_styles,
                                &metrics
                            )
                    })
                },
                _ => {
                    // Unfortunately, we can't assert that inherited_styles, if
                    // present, is pointer-equal to styles.primary(), or even
                    // equal in any meaningful way.  The way it can fail is as
                    // follows.  Say we append an element with a ::before,
                    // ::after, or ::first-line to a parent with a ::first-line,
                    // such that the element ends up on the first line of the
                    // parent (e.g. it's an inline-block in the case it has a
                    // ::first-line, or any container in the ::before/::after
                    // cases).  Then gecko will update its frame's style to
                    // inherit from the parent's ::first-line.  The next time we
                    // try to get the ::before/::after/::first-line style for
                    // the kid, we'll likely pass in the frame's style as
                    // inherited_styles, but that's not pointer-identical to
                    // styles.primary(), because it got reparented.
                    //
                    // Now in practice this turns out to be OK, because all the
                    // cases in which there's a mismatch go ahead and reparent
                    // styles again as needed to make sure the ::first-line
                    // affects all the things it should affect.  But it makes it
                    // impossible to assert anything about the two styles
                    // matching here, unfortunately.
                    styles.pseudos.get(&pseudo).cloned()
                },
            }
        }
        PseudoElementCascadeType::Precomputed => unreachable!("No anonymous boxes"),
        PseudoElementCascadeType::Lazy => {
            debug_assert!(inherited_styles.is_none() ||
                          ptr::eq(inherited_styles.unwrap(),
                                  &**styles.primary()));
            let base = if pseudo.inherits_from_default_values() {
                doc_data.default_computed_values()
            } else {
                styles.primary()
            };
            let guards = StylesheetGuards::same(guard);
            let metrics = get_metrics_provider_for_product();
            doc_data.stylist
                .lazily_compute_pseudo_element_style(
                    &guards,
                    &element,
                    &pseudo,
                    rule_inclusion,
                    base,
                    is_probe,
                    &metrics,
                )
        },
    };

    if is_probe {
        return style;
    }

    Some(style.unwrap_or_else(|| {
        StyleBuilder::for_inheritance(
            doc_data.stylist.device(),
            styles.primary(),
            Some(pseudo),
        ).build()
    }))
}

#[no_mangle]
pub extern "C" fn Servo_ComputedValues_Inherit(
    raw_data: RawServoStyleSetBorrowed,
    pseudo_tag: *mut nsIAtom,
    parent_style_context: ServoStyleContextBorrowedOrNull,
    target: structs::InheritTarget
) -> ServoStyleContextStrong {
    let data = PerDocumentStyleData::from_ffi(raw_data).borrow();

    let for_text = target == structs::InheritTarget::Text;
    let atom = Atom::from(pseudo_tag);
    let pseudo = PseudoElement::from_anon_box_atom(&atom)
        .expect("Not an anon-box? Gah!");
    let style = if let Some(reference) = parent_style_context {
        let mut style = StyleBuilder::for_inheritance(
            data.stylist.device(),
            reference,
            Some(&pseudo)
        );

        if for_text {
            StyleAdjuster::new(&mut style)
                .adjust_for_text();
        }

        style.build()
    } else {
        debug_assert!(!for_text);
        StyleBuilder::for_derived_style(
            data.stylist.device(),
            data.default_computed_values(),
            /* parent_style = */ None,
            Some(&pseudo),
        ).build()
    };

    style.into()
}

#[no_mangle]
pub extern "C" fn Servo_ComputedValues_GetStyleBits(values: ServoStyleContextBorrowed) -> u64 {
    use style::properties::computed_value_flags::*;
    // FIXME(emilio): We could do this more efficiently I'm quite sure.
    let flags = values.flags;
    let mut result = 0;
    if flags.contains(IS_RELEVANT_LINK_VISITED) {
        result |= structs::NS_STYLE_RELEVANT_LINK_VISITED as u64;
    }
    if flags.contains(HAS_TEXT_DECORATION_LINES) {
        result |= structs::NS_STYLE_HAS_TEXT_DECORATION_LINES as u64;
    }
    if flags.contains(SHOULD_SUPPRESS_LINEBREAK) {
        result |= structs::NS_STYLE_SUPPRESS_LINEBREAK as u64;
    }
    if flags.contains(IS_TEXT_COMBINED) {
        result |= structs::NS_STYLE_IS_TEXT_COMBINED as u64;
    }
    if flags.contains(IS_IN_PSEUDO_ELEMENT_SUBTREE) {
        result |= structs::NS_STYLE_HAS_PSEUDO_ELEMENT_DATA as u64;
    }
    if flags.contains(IS_IN_DISPLAY_NONE_SUBTREE) {
        result |= structs::NS_STYLE_IN_DISPLAY_NONE_SUBTREE as u64;
    }
    result
}

#[no_mangle]
pub extern "C" fn Servo_ComputedValues_SpecifiesAnimationsOrTransitions(values: ServoStyleContextBorrowed)
                                                                        -> bool {
    let b = values.get_box();
    b.specifies_animations() || b.specifies_transitions()
}

#[no_mangle]
pub extern "C" fn Servo_ComputedValues_EqualCustomProperties(
    first: ServoComputedDataBorrowed,
    second: ServoComputedDataBorrowed
) -> bool {
    first.get_custom_properties() == second.get_custom_properties()
}

#[no_mangle]
pub extern "C" fn Servo_ComputedValues_GetStyleRuleList(values: ServoStyleContextBorrowed,
                                                        rules: RawGeckoServoStyleRuleListBorrowedMut) {
    let rule_node = match values.rules {
        Some(ref r) => r,
        None => return,
    };

    let global_style_data = &*GLOBAL_STYLE_DATA;
    let guard = global_style_data.shared_lock.read();

    // TODO(emilio): Will benefit from SmallVec.
    let mut result = vec![];
    for node in rule_node.self_and_ancestors() {
        let style_rule = match *node.style_source() {
            StyleSource::Style(ref rule) => rule,
            _ => continue,
        };

        if node.importance().important() {
            let block = style_rule.read_with(&guard).block.read_with(&guard);
            if block.any_normal() {
                // We'll append it when we find the normal rules in our
                // parent chain.
                continue;
            }
        }

        result.push(style_rule);
    }

    unsafe { rules.set_len(result.len() as u32) };
    for (ref src, ref mut dest) in result.into_iter().zip(rules.iter_mut()) {
        src.with_raw_offset_arc(|arc| {
            **dest = *Locked::<StyleRule>::arc_as_borrowed(arc);
        })
    }
}

/// See the comment in `Device` to see why it's ok to pass an owned reference to
/// the pres context (hint: the context outlives the StyleSet, that holds the
/// device alive).
#[no_mangle]
pub extern "C" fn Servo_StyleSet_Init(pres_context: RawGeckoPresContextOwned)
  -> RawServoStyleSetOwned {
    let data = Box::new(PerDocumentStyleData::new(pres_context));
    data.into_ffi()
}

#[no_mangle]
pub extern "C" fn Servo_StyleSet_RebuildCachedData(raw_data: RawServoStyleSetBorrowed) {
    let mut data = PerDocumentStyleData::from_ffi(raw_data).borrow_mut();
    data.stylist.device_mut().rebuild_cached_data();
}

#[no_mangle]
pub extern "C" fn Servo_StyleSet_Clear(raw_data: RawServoStyleSetBorrowed) {
    let mut data = PerDocumentStyleData::from_ffi(raw_data).borrow_mut();
    data.clear_stylist();
}

#[no_mangle]
pub extern "C" fn Servo_StyleSet_Drop(data: RawServoStyleSetOwned) {
    let _ = data.into_box::<PerDocumentStyleData>();
}


/// Updating the stylesheets and redoing selector matching is always happens
/// before the document element is inserted. Therefore we don't need to call
/// `force_dirty` here.
#[no_mangle]
pub extern "C" fn Servo_StyleSet_CompatModeChanged(raw_data: RawServoStyleSetBorrowed) {
    let mut data = PerDocumentStyleData::from_ffi(raw_data).borrow_mut();
    let quirks_mode = unsafe {
        (*data.stylist.device().pres_context().mDocument.raw::<nsIDocument>())
            .mCompatMode
    };

    data.stylist.set_quirks_mode(quirks_mode.into());
}

fn parse_property_into(declarations: &mut SourcePropertyDeclaration,
                       property_id: PropertyId,
                       value: *const nsACString,
                       data: *mut URLExtraData,
                       parsing_mode: structs::ParsingMode,
                       quirks_mode: QuirksMode,
                       reporter: &ParseErrorReporter) -> Result<(), ()> {
    use style_traits::ParsingMode;
    let value = unsafe { value.as_ref().unwrap().as_str_unchecked() };
    let url_data = unsafe { RefPtr::from_ptr_ref(&data) };
    let parsing_mode = ParsingMode::from_bits_truncate(parsing_mode);

    parse_one_declaration_into(
        declarations,
        property_id,
        value,
        url_data,
        reporter,
        parsing_mode,
        quirks_mode)
}

#[no_mangle]
pub extern "C" fn Servo_ParseProperty(property: nsCSSPropertyID, value: *const nsACString,
                                      data: *mut URLExtraData,
                                      parsing_mode: structs::ParsingMode,
                                      quirks_mode: nsCompatibility,
                                      loader: *mut Loader)
                                      -> RawServoDeclarationBlockStrong {
    let id = get_property_id_from_nscsspropertyid!(property,
                                                   RawServoDeclarationBlockStrong::null());
    let mut declarations = SourcePropertyDeclaration::new();
    let reporter = ErrorReporter::new(ptr::null_mut(), loader, data);
    match parse_property_into(&mut declarations, id, value, data,
                              parsing_mode, quirks_mode.into(), &reporter) {
        Ok(()) => {
            let global_style_data = &*GLOBAL_STYLE_DATA;
            let mut block = PropertyDeclarationBlock::new();
            block.extend(declarations.drain(), Importance::Normal);
            Arc::new(global_style_data.shared_lock.wrap(block)).into_strong()
        }
        Err(_) => RawServoDeclarationBlockStrong::null()
    }
}

#[no_mangle]
pub extern "C" fn Servo_ParseEasing(easing: *const nsAString,
                                    data: *mut URLExtraData,
                                    output: nsTimingFunctionBorrowedMut)
                                    -> bool {
    use style::properties::longhands::transition_timing_function;

    let url_data = unsafe { RefPtr::from_ptr_ref(&data) };
    let reporter = NullReporter;
    let context = ParserContext::new(Origin::Author,
                                     url_data,
                                     &reporter,
                                     Some(CssRuleType::Style),
                                     PARSING_MODE_DEFAULT,
                                     QuirksMode::NoQuirks);
    let easing = unsafe { (*easing).to_string() };
    let mut input = ParserInput::new(&easing);
    let mut parser = Parser::new(&mut input);
    let result = transition_timing_function::single_value::parse(&context, &mut parser);
    match result {
        Ok(parsed_easing) => {
            *output = parsed_easing.into();
            true
        },
        Err(_) => false
    }
}

#[no_mangle]
pub extern "C" fn Servo_GetProperties_Overriding_Animation(element: RawGeckoElementBorrowed,
                                                           list: RawGeckoCSSPropertyIDListBorrowed,
                                                           set: nsCSSPropertyIDSetBorrowedMut) {
    let element = GeckoElement(element);
    let element_data = match element.borrow_data() {
        Some(data) => data,
        None => return
    };
    let global_style_data = &*GLOBAL_STYLE_DATA;
    let guard = global_style_data.shared_lock.read();
    let guards = StylesheetGuards::same(&guard);
    let (overridden, custom) =
        element_data.styles.primary().rules().get_properties_overriding_animations(&guards);
    for p in list.iter() {
        match PropertyId::from_nscsspropertyid(*p) {
            Ok(property) => {
                if let PropertyId::Longhand(id) = property {
                    if overridden.contains(id) {
                        unsafe { Gecko_AddPropertyToSet(set, *p) };
                    }
                }
            },
            Err(_) => {
                if *p == nsCSSPropertyID::eCSSPropertyExtra_variable && custom {
                    unsafe { Gecko_AddPropertyToSet(set, *p) };
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn Servo_MatrixTransform_Operate(matrix_operator: MatrixTransformOperator,
                                                from: *const RawGeckoGfxMatrix4x4,
                                                to: *const RawGeckoGfxMatrix4x4,
                                                progress: f64,
                                                output: *mut RawGeckoGfxMatrix4x4) {
    use self::MatrixTransformOperator::{Accumulate, Interpolate};
    use style::properties::longhands::transform::computed_value::ComputedMatrix;

    let from = ComputedMatrix::from(unsafe { from.as_ref() }.expect("not a valid 'from' matrix"));
    let to = ComputedMatrix::from(unsafe { to.as_ref() }.expect("not a valid 'to' matrix"));
    let result = match matrix_operator {
        Interpolate => from.interpolate(&to, progress),
        Accumulate => from.accumulate(&to, progress as u64),
    };

    let output = unsafe { output.as_mut() }.expect("not a valid 'output' matrix");
    if let Ok(result) =  result {
        *output = result.into();
    };
}

#[no_mangle]
pub extern "C" fn Servo_ParseStyleAttribute(data: *const nsACString,
                                            raw_extra_data: *mut URLExtraData,
                                            quirks_mode: nsCompatibility,
                                            loader: *mut Loader)
                                            -> RawServoDeclarationBlockStrong {
    let global_style_data = &*GLOBAL_STYLE_DATA;
    let value = unsafe { data.as_ref().unwrap().as_str_unchecked() };
    let reporter = ErrorReporter::new(ptr::null_mut(), loader, raw_extra_data);
    let url_data = unsafe { RefPtr::from_ptr_ref(&raw_extra_data) };
    Arc::new(global_style_data.shared_lock.wrap(
        GeckoElement::parse_style_attribute(value, url_data, quirks_mode.into(), &reporter)))
        .into_strong()
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_CreateEmpty() -> RawServoDeclarationBlockStrong {
    let global_style_data = &*GLOBAL_STYLE_DATA;
    Arc::new(global_style_data.shared_lock.wrap(PropertyDeclarationBlock::new())).into_strong()
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_Clone(declarations: RawServoDeclarationBlockBorrowed)
                                               -> RawServoDeclarationBlockStrong {
    let global_style_data = &*GLOBAL_STYLE_DATA;
    let guard = global_style_data.shared_lock.read();
    let declarations = Locked::<PropertyDeclarationBlock>::as_arc(&declarations);
    Arc::new(global_style_data.shared_lock.wrap(
        declarations.read_with(&guard).clone()
    )).into_strong()
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_Equals(a: RawServoDeclarationBlockBorrowed,
                                                b: RawServoDeclarationBlockBorrowed)
                                                -> bool {
    let global_style_data = &*GLOBAL_STYLE_DATA;
    let guard = global_style_data.shared_lock.read();
    *Locked::<PropertyDeclarationBlock>::as_arc(&a).read_with(&guard).declarations() ==
    *Locked::<PropertyDeclarationBlock>::as_arc(&b).read_with(&guard).declarations()
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_GetCssText(declarations: RawServoDeclarationBlockBorrowed,
                                                    result: *mut nsAString) {
    read_locked_arc(declarations, |decls: &PropertyDeclarationBlock| {
        decls.to_css(unsafe { result.as_mut().unwrap() }).unwrap();
    })
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_SerializeOneValue(
    declarations: RawServoDeclarationBlockBorrowed,
    property_id: nsCSSPropertyID, buffer: *mut nsAString)
{
    let property_id = get_property_id_from_nscsspropertyid!(property_id, ());
    read_locked_arc(declarations, |decls: &PropertyDeclarationBlock| {
        let mut string = String::new();
        let rv = decls.single_value_to_css(&property_id, &mut string);
        debug_assert!(rv.is_ok());

        let buffer = unsafe { buffer.as_mut().unwrap() };
        buffer.assign_utf8(&string);
    })
}

#[no_mangle]
pub extern "C" fn Servo_SerializeFontValueForCanvas(
    declarations: RawServoDeclarationBlockBorrowed,
    buffer: *mut nsAString) {
    use style::properties::shorthands::font;

    read_locked_arc(declarations, |decls: &PropertyDeclarationBlock| {
        let longhands = match font::LonghandsToSerialize::from_iter(decls.declarations_iter()) {
            Ok(l) => l,
            Err(()) => {
                warn!("Unexpected property!");
                return;
            }
        };

        let mut string = String::new();
        let rv = longhands.to_css_for_canvas(&mut string);
        debug_assert!(rv.is_ok());

        let buffer = unsafe { buffer.as_mut().unwrap() };
        buffer.assign_utf8(&string);
    })
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_Count(declarations: RawServoDeclarationBlockBorrowed) -> u32 {
    read_locked_arc(declarations, |decls: &PropertyDeclarationBlock| {
        decls.declarations().len() as u32
    })
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_GetNthProperty(declarations: RawServoDeclarationBlockBorrowed,
                                                        index: u32, result: *mut nsAString) -> bool {
    read_locked_arc(declarations, |decls: &PropertyDeclarationBlock| {
        if let Some(&(ref decl, _)) = decls.declarations().get(index as usize) {
            let result = unsafe { result.as_mut().unwrap() };
            result.assign_utf8(&decl.id().name());
            true
        } else {
            false
        }
    })
}

macro_rules! get_property_id_from_property {
    ($property: ident, $ret: expr) => {{
        let property = unsafe { $property.as_ref().unwrap().as_str_unchecked() };
        match PropertyId::parse(property.into()) {
            Ok(property_id) => property_id,
            Err(_) => { return $ret; }
        }
    }}
}

fn get_property_value(declarations: RawServoDeclarationBlockBorrowed,
                      property_id: PropertyId, value: *mut nsAString) {
    read_locked_arc(declarations, |decls: &PropertyDeclarationBlock| {
        decls.property_value_to_css(&property_id, unsafe { value.as_mut().unwrap() }).unwrap();
    })
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_GetPropertyValue(declarations: RawServoDeclarationBlockBorrowed,
                                                          property: *const nsACString, value: *mut nsAString) {
    get_property_value(declarations, get_property_id_from_property!(property, ()), value)
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_GetPropertyValueById(declarations: RawServoDeclarationBlockBorrowed,
                                                              property: nsCSSPropertyID, value: *mut nsAString) {
    get_property_value(declarations, get_property_id_from_nscsspropertyid!(property, ()), value)
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_GetPropertyIsImportant(declarations: RawServoDeclarationBlockBorrowed,
                                                                property: *const nsACString) -> bool {
    let property_id = get_property_id_from_property!(property, false);
    read_locked_arc(declarations, |decls: &PropertyDeclarationBlock| {
        decls.property_priority(&property_id).important()
    })
}

fn set_property(declarations: RawServoDeclarationBlockBorrowed, property_id: PropertyId,
                value: *const nsACString, is_important: bool, data: *mut URLExtraData,
                parsing_mode: structs::ParsingMode,
                quirks_mode: QuirksMode,
                loader: *mut Loader) -> bool {
    let mut source_declarations = SourcePropertyDeclaration::new();
    let reporter = ErrorReporter::new(ptr::null_mut(), loader, data);
    match parse_property_into(&mut source_declarations, property_id, value, data,
                              parsing_mode, quirks_mode, &reporter) {
        Ok(()) => {
            let importance = if is_important { Importance::Important } else { Importance::Normal };
            write_locked_arc(declarations, |decls: &mut PropertyDeclarationBlock| {
                decls.extend_reset(source_declarations.drain(), importance)
            })
        },
        Err(_) => false,
    }
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_SetProperty(declarations: RawServoDeclarationBlockBorrowed,
                                                     property: *const nsACString, value: *const nsACString,
                                                     is_important: bool, data: *mut URLExtraData,
                                                     parsing_mode: structs::ParsingMode,
                                                     quirks_mode: nsCompatibility,
                                                     loader: *mut Loader) -> bool {
    set_property(declarations, get_property_id_from_property!(property, false),
                 value, is_important, data, parsing_mode, quirks_mode.into(), loader)
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_SetPropertyById(declarations: RawServoDeclarationBlockBorrowed,
                                                         property: nsCSSPropertyID, value: *const nsACString,
                                                         is_important: bool, data: *mut URLExtraData,
                                                         parsing_mode: structs::ParsingMode,
                                                         quirks_mode: nsCompatibility,
                                                         loader: *mut Loader) -> bool {
    set_property(declarations, get_property_id_from_nscsspropertyid!(property, false),
                 value, is_important, data, parsing_mode, quirks_mode.into(), loader)
}

fn remove_property(declarations: RawServoDeclarationBlockBorrowed, property_id: PropertyId) {
    write_locked_arc(declarations, |decls: &mut PropertyDeclarationBlock| {
        decls.remove_property(&property_id);
    });
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_RemoveProperty(declarations: RawServoDeclarationBlockBorrowed,
                                                        property: *const nsACString) {
    remove_property(declarations, get_property_id_from_property!(property, ()))
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_RemovePropertyById(declarations: RawServoDeclarationBlockBorrowed,
                                                            property: nsCSSPropertyID) {
    remove_property(declarations, get_property_id_from_nscsspropertyid!(property, ()))
}

#[no_mangle]
pub extern "C" fn Servo_MediaList_Create() -> RawServoMediaListStrong {
    let global_style_data = &*GLOBAL_STYLE_DATA;
    Arc::new(global_style_data.shared_lock.wrap(MediaList::empty())).into_strong()
}

#[no_mangle]
pub extern "C" fn Servo_MediaList_DeepClone(list: RawServoMediaListBorrowed) -> RawServoMediaListStrong {
    let global_style_data = &*GLOBAL_STYLE_DATA;
    read_locked_arc(list, |list: &MediaList| {
        Arc::new(global_style_data.shared_lock.wrap(list.clone()))
            .into_strong()
    })
}

#[no_mangle]
pub extern "C" fn Servo_MediaList_Matches(list: RawServoMediaListBorrowed,
                                          raw_data: RawServoStyleSetBorrowed)
                                          -> bool {
    let per_doc_data = PerDocumentStyleData::from_ffi(raw_data).borrow();
    read_locked_arc(list, |list: &MediaList| {
        list.evaluate(per_doc_data.stylist.device(), per_doc_data.stylist.quirks_mode())
    })
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_HasCSSWideKeyword(declarations: RawServoDeclarationBlockBorrowed,
                                                           property: nsCSSPropertyID) -> bool {
    let property_id = get_property_id_from_nscsspropertyid!(property, false);
    read_locked_arc(declarations, |decls: &PropertyDeclarationBlock| {
        decls.has_css_wide_keyword(&property_id)
    })
}

#[no_mangle]
pub extern "C" fn Servo_MediaList_GetText(list: RawServoMediaListBorrowed, result: *mut nsAString) {
    read_locked_arc(list, |list: &MediaList| {
        list.to_css(unsafe { result.as_mut().unwrap() }).unwrap();
    })
}

#[no_mangle]
pub extern "C" fn Servo_MediaList_SetText(list: RawServoMediaListBorrowed, text: *const nsACString) {
    let text = unsafe { text.as_ref().unwrap().as_str_unchecked() };
    let mut input = ParserInput::new(&text);
    let mut parser = Parser::new(&mut input);
    let url_data = unsafe { dummy_url_data() };
    let reporter = NullReporter;
    let context = ParserContext::new_for_cssom(url_data, &reporter, Some(CssRuleType::Media),
                                               PARSING_MODE_DEFAULT,
                                               QuirksMode::NoQuirks);
     write_locked_arc(list, |list: &mut MediaList| {
        *list = parse_media_query_list(&context, &mut parser);
    })
}

#[no_mangle]
pub extern "C" fn Servo_MediaList_GetLength(list: RawServoMediaListBorrowed) -> u32 {
    read_locked_arc(list, |list: &MediaList| list.media_queries.len() as u32)
}

#[no_mangle]
pub extern "C" fn Servo_MediaList_GetMediumAt(list: RawServoMediaListBorrowed, index: u32,
                                              result: *mut nsAString) -> bool {
    read_locked_arc(list, |list: &MediaList| {
        if let Some(media_query) = list.media_queries.get(index as usize) {
            media_query.to_css(unsafe { result.as_mut().unwrap() }).unwrap();
            true
        } else {
            false
        }
    })
}

#[no_mangle]
pub extern "C" fn Servo_MediaList_AppendMedium(list: RawServoMediaListBorrowed,
                                               new_medium: *const nsACString) {
    let new_medium = unsafe { new_medium.as_ref().unwrap().as_str_unchecked() };
    let url_data = unsafe { dummy_url_data() };
    let reporter = NullReporter;
    let context = ParserContext::new_for_cssom(url_data, &reporter, Some(CssRuleType::Media),
                                               PARSING_MODE_DEFAULT,
                                               QuirksMode::NoQuirks);
    write_locked_arc(list, |list: &mut MediaList| {
        list.append_medium(&context, new_medium);
    })
}

#[no_mangle]
pub extern "C" fn Servo_MediaList_DeleteMedium(list: RawServoMediaListBorrowed,
                                               old_medium: *const nsACString) -> bool {
    let old_medium = unsafe { old_medium.as_ref().unwrap().as_str_unchecked() };
    let url_data = unsafe { dummy_url_data() };
    let reporter = NullReporter;
    let context = ParserContext::new_for_cssom(url_data, &reporter, Some(CssRuleType::Media),
                                               PARSING_MODE_DEFAULT,
                                               QuirksMode::NoQuirks);
    write_locked_arc(list, |list: &mut MediaList| list.delete_medium(&context, old_medium))
}

macro_rules! get_longhand_from_id {
    ($id:expr) => {
        match PropertyId::from_nscsspropertyid($id) {
            Ok(PropertyId::Longhand(long)) => long,
            _ => {
                panic!("stylo: unknown presentation property with id {:?}", $id);
            }
        }
    };
}

macro_rules! match_wrap_declared {
    ($longhand:ident, $($property:ident => $inner:expr,)*) => (
        match $longhand {
            $(
                LonghandId::$property => PropertyDeclaration::$property($inner),
            )*
            _ => {
                panic!("stylo: Don't know how to handle presentation property {:?}", $longhand);
            }
        }
    )
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_PropertyIsSet(declarations:
                                                       RawServoDeclarationBlockBorrowed,
                                                       property: nsCSSPropertyID)
        -> bool {
    use style::properties::PropertyDeclarationId;
    let long = get_longhand_from_id!(property);
    read_locked_arc(declarations, |decls: &PropertyDeclarationBlock| {
        decls.get(PropertyDeclarationId::Longhand(long)).is_some()
    })
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_SetIdentStringValue(declarations:
                                                             RawServoDeclarationBlockBorrowed,
                                                             property:
                                                             nsCSSPropertyID,
                                                             value:
                                                             *mut nsIAtom) {
    use style::properties::{PropertyDeclaration, LonghandId};
    use style::properties::longhands::_x_lang::computed_value::T as Lang;

    let long = get_longhand_from_id!(property);
    let prop = match_wrap_declared! { long,
        XLang => Lang(Atom::from(value)),
    };
    write_locked_arc(declarations, |decls: &mut PropertyDeclarationBlock| {
        decls.push(prop, Importance::Normal);
    })
}

#[no_mangle]
#[allow(unreachable_code)]
pub extern "C" fn Servo_DeclarationBlock_SetKeywordValue(declarations:
                                                         RawServoDeclarationBlockBorrowed,
                                                         property: nsCSSPropertyID,
                                                         value: i32) {
    use style::properties::{PropertyDeclaration, LonghandId};
    use style::properties::longhands;
    use style::values::specified::BorderStyle;

    let long = get_longhand_from_id!(property);
    let value = value as u32;

    let prop = match_wrap_declared! { long,
        MozUserModify => longhands::_moz_user_modify::SpecifiedValue::from_gecko_keyword(value),
        // TextEmphasisPosition => FIXME implement text-emphasis-position
        Direction => longhands::direction::SpecifiedValue::from_gecko_keyword(value),
        Display => longhands::display::SpecifiedValue::from_gecko_keyword(value),
        Float => longhands::float::SpecifiedValue::from_gecko_keyword(value),
        VerticalAlign => longhands::vertical_align::SpecifiedValue::from_gecko_keyword(value),
        TextAlign => longhands::text_align::SpecifiedValue::from_gecko_keyword(value),
        TextEmphasisPosition => longhands::text_emphasis_position::SpecifiedValue::from_gecko_keyword(value),
        Clear => longhands::clear::SpecifiedValue::from_gecko_keyword(value),
        FontSize => {
            // We rely on Gecko passing in font-size values (0...7) here.
            longhands::font_size::SpecifiedValue::from_html_size(value as u8)
        },
        FontStyle => longhands::font_style::computed_value::T::from_gecko_keyword(value).into(),
        FontWeight => longhands::font_weight::SpecifiedValue::from_gecko_keyword(value),
        ListStyleType => Box::new(longhands::list_style_type::SpecifiedValue::from_gecko_keyword(value)),
        MozMathVariant => longhands::_moz_math_variant::SpecifiedValue::from_gecko_keyword(value),
        WhiteSpace => longhands::white_space::SpecifiedValue::from_gecko_keyword(value),
        CaptionSide => longhands::caption_side::SpecifiedValue::from_gecko_keyword(value),
        BorderTopStyle => BorderStyle::from_gecko_keyword(value),
        BorderRightStyle => BorderStyle::from_gecko_keyword(value),
        BorderBottomStyle => BorderStyle::from_gecko_keyword(value),
        BorderLeftStyle => BorderStyle::from_gecko_keyword(value),
    };
    write_locked_arc(declarations, |decls: &mut PropertyDeclarationBlock| {
        decls.push(prop, Importance::Normal);
    })
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_SetIntValue(declarations: RawServoDeclarationBlockBorrowed,
                                                     property: nsCSSPropertyID,
                                                     value: i32) {
    use style::properties::{PropertyDeclaration, LonghandId};
    use style::properties::longhands::_moz_script_level::SpecifiedValue as MozScriptLevel;
    use style::properties::longhands::_x_span::computed_value::T as Span;

    let long = get_longhand_from_id!(property);
    let prop = match_wrap_declared! { long,
        XSpan => Span(value),
        // Gecko uses Integer values to signal that it is relative
        MozScriptLevel => MozScriptLevel::Relative(value),
    };
    write_locked_arc(declarations, |decls: &mut PropertyDeclarationBlock| {
        decls.push(prop, Importance::Normal);
    })
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_SetPixelValue(declarations:
                                                       RawServoDeclarationBlockBorrowed,
                                                       property: nsCSSPropertyID,
                                                       value: f32) {
    use style::properties::{PropertyDeclaration, LonghandId};
    use style::properties::longhands::border_spacing::SpecifiedValue as BorderSpacing;
    use style::properties::longhands::height::SpecifiedValue as Height;
    use style::properties::longhands::width::SpecifiedValue as Width;
    use style::values::specified::BorderSideWidth;
    use style::values::specified::MozLength;
    use style::values::specified::length::{NoCalcLength, LengthOrPercentage};

    let long = get_longhand_from_id!(property);
    let nocalc = NoCalcLength::from_px(value);

    let prop = match_wrap_declared! { long,
        Height => Height(MozLength::LengthOrPercentageOrAuto(nocalc.into())),
        Width => Width(MozLength::LengthOrPercentageOrAuto(nocalc.into())),
        BorderTopWidth => BorderSideWidth::Length(nocalc.into()),
        BorderRightWidth => BorderSideWidth::Length(nocalc.into()),
        BorderBottomWidth => BorderSideWidth::Length(nocalc.into()),
        BorderLeftWidth => BorderSideWidth::Length(nocalc.into()),
        MarginTop => nocalc.into(),
        MarginRight => nocalc.into(),
        MarginBottom => nocalc.into(),
        MarginLeft => nocalc.into(),
        PaddingTop => nocalc.into(),
        PaddingRight => nocalc.into(),
        PaddingBottom => nocalc.into(),
        PaddingLeft => nocalc.into(),
        BorderSpacing => Box::new(
            BorderSpacing {
                horizontal: nocalc.into(),
                vertical: None,
            }
        ),
        BorderTopLeftRadius => Box::new(LengthOrPercentage::from(nocalc).into()),
        BorderTopRightRadius => Box::new(LengthOrPercentage::from(nocalc).into()),
        BorderBottomLeftRadius => Box::new(LengthOrPercentage::from(nocalc).into()),
        BorderBottomRightRadius => Box::new(LengthOrPercentage::from(nocalc).into()),
    };
    write_locked_arc(declarations, |decls: &mut PropertyDeclarationBlock| {
        decls.push(prop, Importance::Normal);
    })
}


#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_SetLengthValue(declarations:
                                                        RawServoDeclarationBlockBorrowed,
                                                        property: nsCSSPropertyID,
                                                        value: f32,
                                                        unit: structs::nsCSSUnit) {
    use style::properties::{PropertyDeclaration, LonghandId};
    use style::properties::longhands::_moz_script_min_size::SpecifiedValue as MozScriptMinSize;
    use style::properties::longhands::width::SpecifiedValue as Width;
    use style::values::specified::MozLength;
    use style::values::specified::length::{AbsoluteLength, FontRelativeLength, PhysicalLength};
    use style::values::specified::length::{LengthOrPercentage, NoCalcLength};

    let long = get_longhand_from_id!(property);
    let nocalc = match unit {
        structs::nsCSSUnit::eCSSUnit_EM => NoCalcLength::FontRelative(FontRelativeLength::Em(value)),
        structs::nsCSSUnit::eCSSUnit_XHeight => NoCalcLength::FontRelative(FontRelativeLength::Ex(value)),
        structs::nsCSSUnit::eCSSUnit_Pixel => NoCalcLength::Absolute(AbsoluteLength::Px(value)),
        structs::nsCSSUnit::eCSSUnit_Inch => NoCalcLength::Absolute(AbsoluteLength::In(value)),
        structs::nsCSSUnit::eCSSUnit_Centimeter => NoCalcLength::Absolute(AbsoluteLength::Cm(value)),
        structs::nsCSSUnit::eCSSUnit_Millimeter => NoCalcLength::Absolute(AbsoluteLength::Mm(value)),
        structs::nsCSSUnit::eCSSUnit_PhysicalMillimeter => NoCalcLength::Physical(PhysicalLength(value)),
        structs::nsCSSUnit::eCSSUnit_Point => NoCalcLength::Absolute(AbsoluteLength::Pt(value)),
        structs::nsCSSUnit::eCSSUnit_Pica => NoCalcLength::Absolute(AbsoluteLength::Pc(value)),
        structs::nsCSSUnit::eCSSUnit_Quarter => NoCalcLength::Absolute(AbsoluteLength::Q(value)),
        _ => unreachable!("Unknown unit {:?} passed to SetLengthValue", unit)
    };

    let prop = match_wrap_declared! { long,
        Width => Width(MozLength::LengthOrPercentageOrAuto(nocalc.into())),
        FontSize => LengthOrPercentage::from(nocalc).into(),
        MozScriptMinSize => MozScriptMinSize(nocalc),
    };
    write_locked_arc(declarations, |decls: &mut PropertyDeclarationBlock| {
        decls.push(prop, Importance::Normal);
    })
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_SetNumberValue(declarations:
                                                       RawServoDeclarationBlockBorrowed,
                                                       property: nsCSSPropertyID,
                                                       value: f32) {
    use style::properties::{PropertyDeclaration, LonghandId};
    use style::properties::longhands::_moz_script_level::SpecifiedValue as MozScriptLevel;

    let long = get_longhand_from_id!(property);

    let prop = match_wrap_declared! { long,
        MozScriptSizeMultiplier => value,
        // Gecko uses Number values to signal that it is absolute
        MozScriptLevel => MozScriptLevel::Absolute(value as i32),
    };
    write_locked_arc(declarations, |decls: &mut PropertyDeclarationBlock| {
        decls.push(prop, Importance::Normal);
    })
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_SetPercentValue(declarations:
                                                         RawServoDeclarationBlockBorrowed,
                                                         property: nsCSSPropertyID,
                                                         value: f32) {
    use style::properties::{PropertyDeclaration, LonghandId};
    use style::properties::longhands::height::SpecifiedValue as Height;
    use style::properties::longhands::width::SpecifiedValue as Width;
    use style::values::computed::Percentage;
    use style::values::specified::MozLength;
    use style::values::specified::length::LengthOrPercentage;

    let long = get_longhand_from_id!(property);
    let pc = Percentage(value);

    let prop = match_wrap_declared! { long,
        Height => Height(MozLength::LengthOrPercentageOrAuto(pc.into())),
        Width => Width(MozLength::LengthOrPercentageOrAuto(pc.into())),
        MarginTop => pc.into(),
        MarginRight => pc.into(),
        MarginBottom => pc.into(),
        MarginLeft => pc.into(),
        FontSize => LengthOrPercentage::from(pc).into(),
    };
    write_locked_arc(declarations, |decls: &mut PropertyDeclarationBlock| {
        decls.push(prop, Importance::Normal);
    })
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_SetAutoValue(declarations:
                                                      RawServoDeclarationBlockBorrowed,
                                                      property: nsCSSPropertyID) {
    use style::properties::{PropertyDeclaration, LonghandId};
    use style::properties::longhands::height::SpecifiedValue as Height;
    use style::properties::longhands::width::SpecifiedValue as Width;
    use style::values::specified::LengthOrPercentageOrAuto;

    let long = get_longhand_from_id!(property);
    let auto = LengthOrPercentageOrAuto::Auto;

    let prop = match_wrap_declared! { long,
        Height => Height::auto(),
        Width => Width::auto(),
        MarginTop => auto,
        MarginRight => auto,
        MarginBottom => auto,
        MarginLeft => auto,
    };
    write_locked_arc(declarations, |decls: &mut PropertyDeclarationBlock| {
        decls.push(prop, Importance::Normal);
    })
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_SetCurrentColor(declarations:
                                                         RawServoDeclarationBlockBorrowed,
                                                         property: nsCSSPropertyID) {
    use style::properties::{PropertyDeclaration, LonghandId};
    use style::values::specified::Color;

    let long = get_longhand_from_id!(property);
    let cc = Color::currentcolor();

    let prop = match_wrap_declared! { long,
        BorderTopColor => cc,
        BorderRightColor => cc,
        BorderBottomColor => cc,
        BorderLeftColor => cc,
    };
    write_locked_arc(declarations, |decls: &mut PropertyDeclarationBlock| {
        decls.push(prop, Importance::Normal);
    })
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_SetColorValue(declarations:
                                                       RawServoDeclarationBlockBorrowed,
                                                       property: nsCSSPropertyID,
                                                       value: structs::nscolor) {
    use style::gecko::values::convert_nscolor_to_rgba;
    use style::properties::{PropertyDeclaration, LonghandId};
    use style::properties::longhands;
    use style::values::specified::Color;

    let long = get_longhand_from_id!(property);
    let rgba = convert_nscolor_to_rgba(value);
    let color = Color::rgba(rgba);

    let prop = match_wrap_declared! { long,
        BorderTopColor => color,
        BorderRightColor => color,
        BorderBottomColor => color,
        BorderLeftColor => color,
        Color => longhands::color::SpecifiedValue(color),
        BackgroundColor => color,
    };
    write_locked_arc(declarations, |decls: &mut PropertyDeclarationBlock| {
        decls.push(prop, Importance::Normal);
    })
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_SetFontFamily(declarations:
                                                       RawServoDeclarationBlockBorrowed,
                                                       value: *const nsAString) {
    use cssparser::{Parser, ParserInput};
    use style::properties::PropertyDeclaration;
    use style::properties::longhands::font_family::SpecifiedValue as FontFamily;

    let string = unsafe { (*value).to_string() };
    let mut input = ParserInput::new(&string);
    let mut parser = Parser::new(&mut input);
    let result = FontFamily::parse(&mut parser);
    if let Ok(family) = result {
        if parser.is_exhausted() {
            let decl = PropertyDeclaration::FontFamily(Box::new(family));
            write_locked_arc(declarations, |decls: &mut PropertyDeclarationBlock| {
                decls.push(decl, Importance::Normal);
            })
        }
    }
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_SetBackgroundImage(declarations:
                                                            RawServoDeclarationBlockBorrowed,
                                                            value: *const nsAString,
                                                            raw_extra_data: *mut URLExtraData) {
    use style::properties::PropertyDeclaration;
    use style::properties::longhands::background_image::SpecifiedValue as BackgroundImage;
    use style::values::Either;
    use style::values::generics::image::Image;
    use style::values::specified::url::SpecifiedUrl;

    let url_data = unsafe { RefPtr::from_ptr_ref(&raw_extra_data) };
    let string = unsafe { (*value).to_string() };
    let error_reporter = NullReporter;
    let context = ParserContext::new(Origin::Author, url_data, &error_reporter,
                                     Some(CssRuleType::Style), PARSING_MODE_DEFAULT,
                                     QuirksMode::NoQuirks);
    if let Ok(mut url) = SpecifiedUrl::parse_from_string(string.into(), &context) {
        url.build_image_value();
        let decl = PropertyDeclaration::BackgroundImage(BackgroundImage(
            vec![Either::Second(Image::Url(url))]
        ));
        write_locked_arc(declarations, |decls: &mut PropertyDeclarationBlock| {
            decls.push(decl, Importance::Normal);
        })
    }
}

#[no_mangle]
pub extern "C" fn Servo_DeclarationBlock_SetTextDecorationColorOverride(declarations:
                                                                RawServoDeclarationBlockBorrowed) {
    use style::properties::PropertyDeclaration;
    use style::properties::longhands::text_decoration_line;

    let mut decoration = text_decoration_line::computed_value::none;
    decoration |= text_decoration_line::COLOR_OVERRIDE;
    let decl = PropertyDeclaration::TextDecorationLine(decoration);
    write_locked_arc(declarations, |decls: &mut PropertyDeclarationBlock| {
        decls.push(decl, Importance::Normal);
    })
}

#[no_mangle]
pub extern "C" fn Servo_CSSSupports2(property: *const nsACString,
                                     value: *const nsACString) -> bool {
    let id = get_property_id_from_property!(property, false);

    let mut declarations = SourcePropertyDeclaration::new();
    parse_property_into(
        &mut declarations,
        id,
        value,
        unsafe { DUMMY_URL_DATA },
        structs::ParsingMode_Default,
        QuirksMode::NoQuirks,
        &NullReporter,
    ).is_ok()
}

#[no_mangle]
pub extern "C" fn Servo_CSSSupports(cond: *const nsACString) -> bool {
    let condition = unsafe { cond.as_ref().unwrap().as_str_unchecked() };
    let mut input = ParserInput::new(&condition);
    let mut input = Parser::new(&mut input);
    let cond = input.parse_entirely(|i| parse_condition_or_declaration(i));
    if let Ok(cond) = cond {
        let url_data = unsafe { dummy_url_data() };
        let reporter = NullReporter;
        let context = ParserContext::new_for_cssom(url_data, &reporter, Some(CssRuleType::Style),
                                                   PARSING_MODE_DEFAULT,
                                                   QuirksMode::NoQuirks);
        cond.eval(&context)
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn Servo_NoteExplicitHints(element: RawGeckoElementBorrowed,
                                          restyle_hint: nsRestyleHint,
                                          change_hint: nsChangeHint) {
    GeckoElement(element).note_explicit_hints(restyle_hint, change_hint);
}

#[no_mangle]
pub extern "C" fn Servo_TakeChangeHint(element: RawGeckoElementBorrowed,
                                       raw_flags: ServoTraversalFlags,
                                       was_restyled: *mut bool) -> nsChangeHint
{
    let flags = TraversalFlags::from_bits_truncate(raw_flags);
    let mut was_restyled =  unsafe { was_restyled.as_mut().unwrap() };
    let element = GeckoElement(element);

    let damage = match element.mutate_data() {
        Some(mut data) => {
            *was_restyled = data.restyle.is_restyle();

            let damage = data.restyle.damage;
            if flags.for_animation_only() {
                if !*was_restyled {
                    // Don't touch elements if the element was not restyled
                    // in throttled animation flush.
                    debug!("Skip post traversal for throttled animation flush {:?} restyle={:?}",
                           element, data.restyle);
                    return nsChangeHint(0);
                }
                // In the case where we call this function for post traversal for
                // flusing throttled animations (i.e. without normal restyle
                // traversal), we need to preserve restyle hints for normal restyle
                // traversal. Restyle hints for animations have been already
                // removed during animation-only traversal.
                debug_assert!(!data.restyle.hint.has_animation_hint(),
                              "Animation restyle hints should have been already removed");
                data.clear_restyle_flags_and_damage();
            } else {
                data.clear_restyle_state();
            }
            damage
        }
        None => {
            warn!("Trying to get change hint from unstyled element");
            *was_restyled = false;
            GeckoRestyleDamage::empty()
        }
    };

    debug!("Servo_TakeChangeHint: {:?}, damage={:?}", element, damage);
    damage.as_change_hint()
}

#[no_mangle]
pub extern "C" fn Servo_ResolveStyle(element: RawGeckoElementBorrowed,
                                     _raw_data: RawServoStyleSetBorrowed,
                                     raw_flags: ServoTraversalFlags)
                                     -> ServoStyleContextStrong
{
    let flags = TraversalFlags::from_bits_truncate(raw_flags);
    let element = GeckoElement(element);
    debug!("Servo_ResolveStyle: {:?}", element);
    let data =
        element.borrow_data().expect("Resolving style on unstyled element");

    // TODO(emilio): Downgrade to debug assertions when close to release.
    assert!(data.has_styles(), "Resolving style on unstyled element");
    // In the case where we process for throttled animation, there remaings
    // restyle hints other than animation hints.
    debug_assert!(element.has_current_styles_for_traversal(&*data, flags),
                  "Resolving style on {:?} without current styles: {:?}", element, data);
    data.styles.primary().clone().into()
}

#[no_mangle]
pub extern "C" fn Servo_ResolveStyleAllowStale(element: RawGeckoElementBorrowed)
                                               -> ServoStyleContextStrong
{
    let element = GeckoElement(element);
    debug!("Servo_ResolveStyleAllowStale: {:?}", element);
    let data =
        element.borrow_data().expect("Resolving style on unstyled element");
    assert!(data.has_styles(), "Resolving style on unstyled element");
    data.styles.primary().clone().into()
}

#[no_mangle]
pub extern "C" fn Servo_ResolveStyleLazily(element: RawGeckoElementBorrowed,
                                           pseudo_type: CSSPseudoElementType,
                                           rule_inclusion: StyleRuleInclusion,
                                           snapshots: *const ServoElementSnapshotTable,
                                           raw_data: RawServoStyleSetBorrowed,
                                           ignore_existing_styles: bool)
     -> ServoStyleContextStrong
{
    debug_assert!(!snapshots.is_null());
    let global_style_data = &*GLOBAL_STYLE_DATA;
    let guard = global_style_data.shared_lock.read();
    let element = GeckoElement(element);
    let doc_data = PerDocumentStyleData::from_ffi(raw_data);
    let data = doc_data.borrow();
    let rule_inclusion = RuleInclusion::from(rule_inclusion);
    let finish = |styles: &ElementStyles| -> Arc<ComputedValues> {
        match PseudoElement::from_pseudo_type(pseudo_type) {
            Some(ref pseudo) => {
                get_pseudo_style(
                    &guard,
                    element,
                    pseudo,
                    rule_inclusion,
                    styles,
                    /* inherited_styles = */ None,
                    &*data,
                    /* is_probe = */ false,
                ).expect("We're not probing, so we should always get a style \
                         back")
            }
            None => styles.primary().clone(),
        }
    };

    // In the common case we already have the style. Check that before setting
    // up all the computation machinery. (Don't use it when we're getting
    // default styles or in a bfcached document (as indicated by
    // ignore_existing_styles), though.)
    if rule_inclusion == RuleInclusion::All && !ignore_existing_styles {
        let styles = element.mutate_data().and_then(|d| {
            if d.has_styles() {
                Some(finish(&d.styles))
            } else {
                None
            }
        });
        if let Some(result) = styles {
            return result.into();
        }
    }

    // We don't have the style ready. Go ahead and compute it as necessary.
    let shared = create_shared_context(&global_style_data,
                                       &guard,
                                       &data,
                                       TraversalFlags::empty(),
                                       unsafe { &*snapshots });
    let mut tlc = ThreadLocalStyleContext::new(&shared);
    let mut context = StyleContext {
        shared: &shared,
        thread_local: &mut tlc,
    };

    let styles = resolve_style(&mut context, element, rule_inclusion, ignore_existing_styles);
    finish(&styles).into()
}

#[no_mangle]
pub extern "C" fn Servo_ReparentStyle(style_to_reparent: ServoStyleContextBorrowed,
                                      parent_style: ServoStyleContextBorrowed,
                                      parent_style_ignoring_first_line: ServoStyleContextBorrowed,
                                      layout_parent_style: ServoStyleContextBorrowed,
                                      element: RawGeckoElementBorrowedOrNull,
                                      raw_data: RawServoStyleSetBorrowed)
     -> ServoStyleContextStrong
{
    let global_style_data = &*GLOBAL_STYLE_DATA;
    let guard = global_style_data.shared_lock.read();
    let doc_data = PerDocumentStyleData::from_ffi(raw_data).borrow();
    let inputs = CascadeInputs::new_from_style(style_to_reparent);
    let metrics = get_metrics_provider_for_product();
    let pseudo = style_to_reparent.pseudo();
    let element = element.map(GeckoElement);

    let mut cascade_flags = CascadeFlags::empty();
    if style_to_reparent.is_anon_box() {
        cascade_flags.insert(SKIP_ROOT_AND_ITEM_BASED_DISPLAY_FIXUP);
    }
    if let Some(element) = element {
        if element.is_link() {
            cascade_flags.insert(IS_LINK);
            if element.is_visited_link() &&
                doc_data.visited_styles_enabled() {
                cascade_flags.insert(IS_VISITED_LINK);
            }
        };

        if element.is_native_anonymous() {
            cascade_flags.insert(PROHIBIT_DISPLAY_CONTENTS);
        }
    }
    if let Some(pseudo) = pseudo.as_ref() {
        cascade_flags.insert(PROHIBIT_DISPLAY_CONTENTS);
        if pseudo.is_fieldset_content() {
            cascade_flags.insert(IS_FIELDSET_CONTENT);
        }
    }

    doc_data.stylist
        .compute_style_with_inputs(&inputs,
                                   pseudo.as_ref(),
                                   &StylesheetGuards::same(&guard),
                                   parent_style,
                                   parent_style_ignoring_first_line,
                                   layout_parent_style,
                                   &metrics,
                                   cascade_flags)
        .into()
}

#[cfg(feature = "gecko_debug")]
fn simulate_compute_values_failure(property: &PropertyValuePair) -> bool {
    let p = property.mProperty;
    let id = get_property_id_from_nscsspropertyid!(p, false);
    id.as_shorthand().is_ok() && property.mSimulateComputeValuesFailure
}

#[cfg(not(feature = "gecko_debug"))]
fn simulate_compute_values_failure(_: &PropertyValuePair) -> bool {
    false
}

fn create_context<'a>(
    per_doc_data: &'a PerDocumentStyleDataImpl,
    font_metrics_provider: &'a FontMetricsProvider,
    style: &'a ComputedValues,
    parent_style: Option<&'a ComputedValues>,
    pseudo: Option<&'a PseudoElement>,
    for_smil_animation: bool,
) -> Context<'a> {
    Context {
        is_root_element: false,
        builder: StyleBuilder::for_derived_style(
            per_doc_data.stylist.device(),
            style,
            parent_style,
            pseudo,
        ),
        font_metrics_provider: font_metrics_provider,
        cached_system_font: None,
        in_media_query: false,
        quirks_mode: per_doc_data.stylist.quirks_mode(),
        for_smil_animation,
    }
}

struct PropertyAndIndex {
    property: PropertyId,
    index: usize,
}

struct PrioritizedPropertyIter<'a> {
    properties: &'a nsTArray<PropertyValuePair>,
    sorted_property_indices: Vec<PropertyAndIndex>,
    curr: usize,
}

impl<'a> PrioritizedPropertyIter<'a> {
    pub fn new(properties: &'a nsTArray<PropertyValuePair>) -> PrioritizedPropertyIter {
        // If we fail to convert a nsCSSPropertyID into a PropertyId we shouldn't fail outright
        // but instead by treating that property as the 'all' property we make it sort last.
        let all = PropertyId::Shorthand(ShorthandId::All);

        let mut sorted_property_indices: Vec<PropertyAndIndex> =
            properties.iter().enumerate().map(|(index, pair)| {
                PropertyAndIndex {
                    property: PropertyId::from_nscsspropertyid(pair.mProperty)
                              .unwrap_or(all.clone()),
                    index,
                }
            }).collect();
        sorted_property_indices.sort_by(|a, b| compare_property_priority(&a.property, &b.property));

        PrioritizedPropertyIter {
            properties,
            sorted_property_indices,
            curr: 0,
        }
    }
}

impl<'a> Iterator for PrioritizedPropertyIter<'a> {
    type Item = &'a PropertyValuePair;

    fn next(&mut self) -> Option<&'a PropertyValuePair> {
        if self.curr >= self.sorted_property_indices.len() {
            return None
        }
        self.curr += 1;
        Some(&self.properties[self.sorted_property_indices[self.curr - 1].index])
    }
}

#[no_mangle]
pub extern "C" fn Servo_GetComputedKeyframeValues(keyframes: RawGeckoKeyframeListBorrowed,
                                                  element: RawGeckoElementBorrowed,
                                                  style: ServoStyleContextBorrowed,
                                                  raw_data: RawServoStyleSetBorrowed,
                                                  computed_keyframes: RawGeckoComputedKeyframeValuesListBorrowedMut)
{
    use std::mem;
    use style::properties::LonghandIdSet;

    let data = PerDocumentStyleData::from_ffi(raw_data).borrow();
    let metrics = get_metrics_provider_for_product();

    let element = GeckoElement(element);
    let parent_element = element.inheritance_parent();
    let parent_data = parent_element.as_ref().and_then(|e| e.borrow_data());
    let parent_style = parent_data.as_ref().map(|d| d.styles.primary()).map(|x| &**x);

    let pseudo = style.pseudo();
    let mut context = create_context(
        &data,
        &metrics,
        &style,
        parent_style,
        pseudo.as_ref(),
        /* for_smil_animation = */ false,
    );

    let global_style_data = &*GLOBAL_STYLE_DATA;
    let guard = global_style_data.shared_lock.read();
    let default_values = data.default_computed_values();

    for (index, keyframe) in keyframes.iter().enumerate() {
        let ref mut animation_values = computed_keyframes[index];

        let mut seen = LonghandIdSet::new();

        let mut property_index = 0;
        for property in PrioritizedPropertyIter::new(&keyframe.mPropertyValues) {
            if simulate_compute_values_failure(property) {
                continue;
            }

            let mut maybe_append_animation_value = |property: AnimatableLonghand,
                                                    value: Option<AnimationValue>| {
                if seen.has_animatable_longhand_bit(&property) {
                    return;
                }
                seen.set_animatable_longhand_bit(&property);

                // This is safe since we immediately write to the uninitialized values.
                unsafe { animation_values.set_len((property_index + 1) as u32) };
                animation_values[property_index].mProperty = (&property).into();
                // We only make sure we have enough space for this variable,
                // but didn't construct a default value for StyleAnimationValue,
                // so we should zero it to avoid getting undefined behaviors.
                animation_values[property_index].mValue.mGecko = unsafe { mem::zeroed() };
                match value {
                    Some(v) => {
                        animation_values[property_index].mValue.mServo.set_arc_leaky(Arc::new(v));
                    },
                    None => {
                        animation_values[property_index].mValue.mServo.mRawPtr = ptr::null_mut();
                    },
                }
                property_index += 1;
            };

            if property.mServoDeclarationBlock.mRawPtr.is_null() {
                let animatable_longhand =
                    AnimatableLonghand::from_nscsspropertyid(property.mProperty);
                // |keyframes.mPropertyValues| should only contain animatable
                // properties, but we check the result from_nscsspropertyid
                // just in case.
                if let Some(property) = animatable_longhand {
                    maybe_append_animation_value(property, None);
                }
                continue;
            }

            let declarations = unsafe { &*property.mServoDeclarationBlock.mRawPtr.clone() };
            let declarations = Locked::<PropertyDeclarationBlock>::as_arc(&declarations);
            let guard = declarations.read_with(&guard);

            for anim in guard.to_animation_value_iter(&mut context, &default_values) {
                maybe_append_animation_value(anim.0, Some(anim.1));
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn Servo_GetAnimationValues(declarations: RawServoDeclarationBlockBorrowed,
                                           element: RawGeckoElementBorrowed,
                                           style: ServoStyleContextBorrowed,
                                           raw_data: RawServoStyleSetBorrowed,
                                           animation_values: RawGeckoServoAnimationValueListBorrowedMut) {
    let data = PerDocumentStyleData::from_ffi(raw_data).borrow();
    let metrics = get_metrics_provider_for_product();

    let element = GeckoElement(element);
    let parent_element = element.inheritance_parent();
    let parent_data = parent_element.as_ref().and_then(|e| e.borrow_data());
    let parent_style = parent_data.as_ref().map(|d| d.styles.primary()).map(|x| &**x);

    let pseudo = style.pseudo();
    let mut context = create_context(
        &data,
        &metrics,
        &style,
        parent_style,
        pseudo.as_ref(),
        /* for_smil_animation = */ true
    );

    let default_values = data.default_computed_values();
    let global_style_data = &*GLOBAL_STYLE_DATA;
    let guard = global_style_data.shared_lock.read();

    let declarations = Locked::<PropertyDeclarationBlock>::as_arc(&declarations);
    let guard = declarations.read_with(&guard);
    for (index, anim) in guard.to_animation_value_iter(&mut context, &default_values).enumerate() {
        unsafe { animation_values.set_len((index + 1) as u32) };
        animation_values[index].set_arc_leaky(Arc::new(anim.1));
    }
}

#[no_mangle]
pub extern "C" fn Servo_AnimationValue_Compute(element: RawGeckoElementBorrowed,
                                               declarations: RawServoDeclarationBlockBorrowed,
                                               style: ServoStyleContextBorrowed,
                                               raw_data: RawServoStyleSetBorrowed)
                                               -> RawServoAnimationValueStrong {
    let data = PerDocumentStyleData::from_ffi(raw_data).borrow();
    let metrics = get_metrics_provider_for_product();

    let element = GeckoElement(element);
    let parent_element = element.inheritance_parent();
    let parent_data = parent_element.as_ref().and_then(|e| e.borrow_data());
    let parent_style = parent_data.as_ref().map(|d| d.styles.primary()).map(|x| &**x);

    let pseudo = style.pseudo();
    let mut context = create_context(
        &data,
        &metrics,
        style,
        parent_style,
        pseudo.as_ref(),
        /* for_smil_animation = */ false
    );

    let default_values = data.default_computed_values();
    let global_style_data = &*GLOBAL_STYLE_DATA;
    let guard = global_style_data.shared_lock.read();
    let declarations = Locked::<PropertyDeclarationBlock>::as_arc(&declarations);
    // We only compute the first element in declarations.
    match declarations.read_with(&guard).declarations().first() {
        Some(&(ref decl, imp)) if imp == Importance::Normal => {
            let animation = AnimationValue::from_declaration(decl, &mut context, default_values);
            animation.map_or(RawServoAnimationValueStrong::null(), |value| {
                Arc::new(value).into_strong()
            })
        },
        _ => RawServoAnimationValueStrong::null()
    }
}

#[no_mangle]
pub extern "C" fn Servo_AssertTreeIsClean(root: RawGeckoElementBorrowed) {
    if !cfg!(feature = "gecko_debug") {
        panic!("Calling Servo_AssertTreeIsClean in release build");
    }

    let root = GeckoElement(root);
    fn assert_subtree_is_clean<'le>(el: GeckoElement<'le>) {
        debug_assert!(!el.has_dirty_descendants() && !el.has_animation_only_dirty_descendants(),
                      "{:?} has still dirty bit {:?} or animation-only dirty bit {:?}",
                      el, el.has_dirty_descendants(), el.has_animation_only_dirty_descendants());
        for child in el.as_node().traversal_children() {
            if let Some(child) = child.as_element() {
                assert_subtree_is_clean(child);
            }
        }
    }

    assert_subtree_is_clean(root);
}

enum Offset {
    Zero,
    One
}

fn fill_in_missing_keyframe_values(all_properties:  &[AnimatableLonghand],
                                   timing_function: nsTimingFunctionBorrowed,
                                   properties_set_at_offset: &LonghandIdSet,
                                   offset: Offset,
                                   keyframes: RawGeckoKeyframeListBorrowedMut) {
    let needs_filling = all_properties.iter().any(|ref property| {
        !properties_set_at_offset.has_animatable_longhand_bit(property)
    });

    // Return earli if all animated properties are already set.
    if !needs_filling {
        return;
    }

    let keyframe = match offset {
        Offset::Zero => unsafe {
            Gecko_GetOrCreateInitialKeyframe(keyframes, timing_function)
        },
        Offset::One => unsafe {
            Gecko_GetOrCreateFinalKeyframe(keyframes, timing_function)
        },
    };

    // Append properties that have not been set at this offset.
    for ref property in all_properties.iter() {
        if !properties_set_at_offset.has_animatable_longhand_bit(property) {
            unsafe { Gecko_AppendPropertyValuePair(&mut (*keyframe).mPropertyValues,
                                                   (*property).into()); }
        }
    }
}

#[no_mangle]
pub extern "C" fn Servo_StyleSet_GetKeyframesForName(raw_data: RawServoStyleSetBorrowed,
                                                     name: *const nsACString,
                                                     inherited_timing_function: nsTimingFunctionBorrowed,
                                                     keyframes: RawGeckoKeyframeListBorrowedMut) -> bool {
    debug_assert!(keyframes.len() == 0,
                  "keyframes should be initially empty");

    let data = PerDocumentStyleData::from_ffi(raw_data).borrow();
    let name = unsafe { Atom::from(name.as_ref().unwrap().as_str_unchecked()) };

    let animation = match data.stylist.animations().get(&name) {
        Some(animation) => animation,
        None => return false,
    };

    let global_style_data = &*GLOBAL_STYLE_DATA;
    let guard = global_style_data.shared_lock.read();

    let mut properties_set_at_current_offset = LonghandIdSet::new();
    let mut properties_set_at_start = LonghandIdSet::new();
    let mut properties_set_at_end = LonghandIdSet::new();
    let mut has_complete_initial_keyframe = false;
    let mut has_complete_final_keyframe = false;
    let mut current_offset = -1.;

    // Iterate over the keyframe rules backwards so we can drop overridden
    // properties (since declarations in later rules override those in earlier
    // ones).
    for step in animation.steps.iter().rev() {
        if step.start_percentage.0 != current_offset {
            properties_set_at_current_offset.clear();
            current_offset = step.start_percentage.0;
        }

        // Override timing_function if the keyframe has an animation-timing-function.
        let timing_function = match step.get_animation_timing_function(&guard) {
            Some(val) => val.into(),
            None => *inherited_timing_function,
        };

        // Look for an existing keyframe with the same offset and timing
        // function or else add a new keyframe at the beginning of the keyframe
        // array.
        let keyframe = unsafe {
            Gecko_GetOrCreateKeyframeAtStart(keyframes,
                                             step.start_percentage.0 as f32,
                                             &timing_function)
        };

        match step.value {
            KeyframesStepValue::ComputedValues => {
                // In KeyframesAnimation::from_keyframes if there is no 0% or
                // 100% keyframe at all, we will create a 'ComputedValues' step
                // to represent that all properties animated by the keyframes
                // animation should be set to the underlying computed value for
                // that keyframe.
                for property in animation.properties_changed.iter() {
                    unsafe {
                        Gecko_AppendPropertyValuePair(&mut (*keyframe).mPropertyValues,
                                                      property.into());
                    }
                }
                if current_offset == 0.0 {
                    has_complete_initial_keyframe = true;
                } else if current_offset == 1.0 {
                    has_complete_final_keyframe = true;
                }
            },
            KeyframesStepValue::Declarations { ref block } => {
                let guard = block.read_with(&guard);
                // Filter out non-animatable properties.
                let animatable =
                    guard.declarations()
                         .iter()
                         .filter(|&&(ref declaration, _)| {
                             declaration.is_animatable()
                         });

                for &(ref declaration, _) in animatable {
                    let property = AnimatableLonghand::from_declaration(declaration).unwrap();
                    // Skip the 'display' property because although it is animatable from SMIL,
                    // it should not be animatable from CSS Animations.
                    if property != AnimatableLonghand::Display &&
                        !properties_set_at_current_offset.has_animatable_longhand_bit(&property) {
                        properties_set_at_current_offset.set_animatable_longhand_bit(&property);
                        if current_offset == 0.0 {
                            properties_set_at_start.set_animatable_longhand_bit(&property);
                        } else if current_offset == 1.0 {
                            properties_set_at_end.set_animatable_longhand_bit(&property);
                        }

                        let property = AnimatableLonghand::from_declaration(declaration).unwrap();
                        unsafe {
                            let pair =
                                Gecko_AppendPropertyValuePair(&mut (*keyframe).mPropertyValues,
                                                              (&property).into());
                            (*pair).mServoDeclarationBlock.set_arc_leaky(
                                Arc::new(global_style_data.shared_lock.wrap(
                                    PropertyDeclarationBlock::with_one(
                                        declaration.clone(), Importance::Normal
                                ))));
                        }
                    }
                }
            },
        }
    }

    // Append property values that are missing in the initial or the final keyframes.
    if !has_complete_initial_keyframe {
        fill_in_missing_keyframe_values(&animation.properties_changed,
                                        inherited_timing_function,
                                        &properties_set_at_start,
                                        Offset::Zero,
                                        keyframes);
    }
    if !has_complete_final_keyframe {
        fill_in_missing_keyframe_values(&animation.properties_changed,
                                        inherited_timing_function,
                                        &properties_set_at_end,
                                        Offset::One,
                                        keyframes);
    }
    true
}

#[no_mangle]
pub extern "C" fn Servo_StyleSet_GetFontFaceRules(raw_data: RawServoStyleSetBorrowed,
                                                  rules: RawGeckoFontFaceRuleListBorrowedMut) {
    let data = PerDocumentStyleData::from_ffi(raw_data).borrow();
    debug_assert!(rules.len() == 0);

    let global_style_data = &*GLOBAL_STYLE_DATA;
    let guard = global_style_data.shared_lock.read();

    unsafe { rules.set_len(data.font_faces.len() as u32) };
    for (src, dest) in data.font_faces.iter().zip(rules.iter_mut()) {
        dest.mRule = src.0.read_with(&guard).clone().forget();
        dest.mSheetType = src.1.into();
    }
}

#[no_mangle]
pub extern "C" fn Servo_StyleSet_GetCounterStyleRule(raw_data: RawServoStyleSetBorrowed,
                                                     name: *mut nsIAtom) -> *mut nsCSSCounterStyleRule {
    let data = PerDocumentStyleData::from_ffi(raw_data).borrow();
    unsafe {
        Atom::with(name, |name| data.counter_styles.get(name))
    }.map(|rule| {
        let global_style_data = &*GLOBAL_STYLE_DATA;
        let guard = global_style_data.shared_lock.read();
        rule.read_with(&guard).get()
    }).unwrap_or(ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn Servo_StyleSet_ResolveForDeclarations(
    raw_data: RawServoStyleSetBorrowed,
    parent_style_context: ServoStyleContextBorrowedOrNull,
    declarations: RawServoDeclarationBlockBorrowed,
) -> ServoStyleContextStrong {
    let doc_data = PerDocumentStyleData::from_ffi(raw_data).borrow();
    let global_style_data = &*GLOBAL_STYLE_DATA;
    let guard = global_style_data.shared_lock.read();
    let guards = StylesheetGuards::same(&guard);

    let parent_style = match parent_style_context {
        Some(parent) => &*parent,
        None => doc_data.default_computed_values(),
    };

    let declarations = Locked::<PropertyDeclarationBlock>::as_arc(&declarations);

    doc_data.stylist.compute_for_declarations(
        &guards,
        parent_style,
        declarations.clone_arc(),
    ).into()
}

#[no_mangle]
pub extern "C" fn Servo_StyleSet_MightHaveAttributeDependency(
    raw_data: RawServoStyleSetBorrowed,
    element: RawGeckoElementBorrowed,
    local_name: *mut nsIAtom,
) -> bool {
    let data = PerDocumentStyleData::from_ffi(raw_data).borrow();
    let element = GeckoElement(element);
    let mut has_dep = false;

    unsafe {
        Atom::with(local_name, |atom| {
            has_dep = data.stylist.might_have_attribute_dependency(atom);

            if !has_dep {
                // TODO(emilio): Consider optimizing this storing attribute
                // dependencies from UA sheets separately, so we could optimize
                // the above lookup if cut_off_inheritance is true.
                element.each_xbl_stylist(|stylist| {
                    has_dep =
                        has_dep || stylist.might_have_attribute_dependency(atom);
                });
            }
        })
    }

    has_dep
}

#[no_mangle]
pub extern "C" fn Servo_StyleSet_HasStateDependency(
    raw_data: RawServoStyleSetBorrowed,
    element: RawGeckoElementBorrowed,
    state: u64,
) -> bool {
    let element = GeckoElement(element);

    let state = ElementState::from_bits_truncate(state);
    let data = PerDocumentStyleData::from_ffi(raw_data).borrow();

    let mut has_dep = data.stylist.might_have_state_dependency(state);
    if !has_dep {
        // TODO(emilio): Consider optimizing this storing attribute
        // dependencies from UA sheets separately, so we could optimize
        // the above lookup if cut_off_inheritance is true.
        element.each_xbl_stylist(|stylist| {
            has_dep = has_dep || stylist.might_have_state_dependency(state);
        });
    }

    has_dep
}

#[no_mangle]
pub extern "C" fn Servo_GetCustomPropertyValue(computed_values: ServoStyleContextBorrowed,
                                               name: *const nsAString,
                                               value: *mut nsAString) -> bool {
    let custom_properties = match computed_values.custom_properties() {
        Some(p) => p,
        None => return false,
    };

    let name = unsafe { Atom::from((&*name)) };
    let computed_value = match custom_properties.get(&name) {
        Some(v) => v,
        None => return false,
    };

    computed_value.to_css(unsafe { value.as_mut().unwrap() }).unwrap();
    true
}

#[no_mangle]
pub extern "C" fn Servo_GetCustomPropertiesCount(computed_values: ServoStyleContextBorrowed) -> u32 {
    match computed_values.custom_properties() {
        Some(p) => p.len() as u32,
        None => 0,
    }
}

#[no_mangle]
pub extern "C" fn Servo_GetCustomPropertyNameAt(computed_values: ServoStyleContextBorrowed,
                                                index: u32,
                                                name: *mut nsAString) -> bool {
    let custom_properties = match computed_values.custom_properties() {
        Some(p) => p,
        None => return false,
    };

    let property_name = match custom_properties.get_key_at(index) {
        Some(n) => n,
        None => return false,
    };

    let name = unsafe { name.as_mut().unwrap() };
    name.assign(&*property_name.as_slice());

    true
}
