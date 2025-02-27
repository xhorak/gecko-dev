/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Specified values.
//!
//! TODO(emilio): Enhance docs.

use Namespace;
use context::QuirksMode;
use cssparser::{Parser, Token, serialize_identifier, BasicParseError};
use parser::{ParserContext, Parse};
use self::url::SpecifiedUrl;
use std::ascii::AsciiExt;
use std::f32;
use std::fmt;
use style_traits::{ToCss, ParseError, StyleParseError};
use style_traits::values::specified::AllowedNumericType;
use super::{Auto, CSSFloat, CSSInteger, Either, None_};
use super::computed::{self, Context, ToComputedValue};
use super::generics::{GreaterThanOrEqualToOne, NonNegative};
use super::generics::grid::{TrackBreadth as GenericTrackBreadth, TrackSize as GenericTrackSize};
use super::generics::grid::TrackList as GenericTrackList;
use values::computed::ComputedValueAsSpecified;
use values::specified::calc::CalcNode;

pub use properties::animated_properties::TransitionProperty;
#[cfg(feature = "gecko")]
pub use self::align::{AlignItems, AlignJustifyContent, AlignJustifySelf, JustifyItems};
pub use self::background::BackgroundSize;
pub use self::border::{BorderCornerRadius, BorderImageSlice, BorderImageWidth};
pub use self::border::{BorderImageSideWidth, BorderRadius, BorderSideWidth};
pub use self::color::{Color, RGBAColor};
pub use self::effects::{BoxShadow, Filter, SimpleShadow};
pub use self::flex::FlexBasis;
#[cfg(feature = "gecko")]
pub use self::gecko::ScrollSnapPoint;
pub use self::image::{ColorStop, EndingShape as GradientEndingShape, Gradient};
pub use self::image::{GradientItem, GradientKind, Image, ImageLayer, MozImageRect};
pub use self::length::{AbsoluteLength, CalcLengthOrPercentage, CharacterWidth};
pub use self::length::{FontRelativeLength, Length, LengthOrNone, LengthOrNumber};
pub use self::length::{LengthOrPercentage, LengthOrPercentageOrAuto};
pub use self::length::{LengthOrPercentageOrNone, MaxLength, MozLength};
pub use self::length::{NoCalcLength, ViewportPercentageLength};
pub use self::length::NonNegativeLengthOrPercentage;
pub use self::rect::LengthOrNumberRect;
pub use self::position::{Position, PositionComponent};
pub use self::svg::{SVGLength, SVGOpacity, SVGPaint, SVGPaintKind, SVGStrokeDashArray, SVGWidth};
pub use self::text::{InitialLetter, LetterSpacing, LineHeight, WordSpacing};
pub use self::transform::{TimingFunction, TransformOrigin};
pub use super::generics::grid::GridLine;
pub use super::generics::grid::GridTemplateComponent as GenericGridTemplateComponent;

#[cfg(feature = "gecko")]
pub mod align;
pub mod background;
pub mod basic_shape;
pub mod border;
pub mod calc;
pub mod color;
pub mod effects;
pub mod flex;
#[cfg(feature = "gecko")]
pub mod gecko;
pub mod grid;
pub mod image;
pub mod length;
pub mod position;
pub mod rect;
pub mod svg;
pub mod text;
pub mod transform;

/// Common handling for the specified value CSS url() values.
pub mod url {
use cssparser::Parser;
use parser::{Parse, ParserContext};
use style_traits::ParseError;
use values::computed::ComputedValueAsSpecified;

#[cfg(feature = "servo")]
pub use ::servo::url::*;
#[cfg(feature = "gecko")]
pub use ::gecko::url::*;

impl Parse for SpecifiedUrl {
    fn parse<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i>> {
        let url = input.expect_url()?;
        Self::parse_from_string(url.as_ref().to_owned(), context)
    }
}

impl Eq for SpecifiedUrl {}

// TODO(emilio): Maybe consider ComputedUrl to save a word in style structs?
impl ComputedValueAsSpecified for SpecifiedUrl {}

no_viewport_percentage!(SpecifiedUrl);
}

/// Parse an `<integer>` value, handling `calc()` correctly.
pub fn parse_integer<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>)
                             -> Result<Integer, ParseError<'i>> {
    // FIXME: remove early returns when lifetimes are non-lexical
    match *input.next()? {
        Token::Number { int_value: Some(v), .. } => return Ok(Integer::new(v)),
        Token::Function(ref name) if name.eq_ignore_ascii_case("calc") => {}
        ref t => return Err(BasicParseError::UnexpectedToken(t.clone()).into())
    }

    let result = input.parse_nested_block(|i| {
        CalcNode::parse_integer(context, i)
    })?;

    Ok(Integer::from_calc(result))
}

/// Parse a `<number>` value, handling `calc()` correctly, and without length
/// limitations.
pub fn parse_number<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>)
                            -> Result<Number, ParseError<'i>> {
    parse_number_with_clamping_mode(context, input, AllowedNumericType::All)
}

/// Parse a `<number>` value, with a given clamping mode.
pub fn parse_number_with_clamping_mode<'i, 't>(context: &ParserContext,
                                               input: &mut Parser<'i, 't>,
                                               clamping_mode: AllowedNumericType)
                                               -> Result<Number, ParseError<'i>> {
    // FIXME: remove early returns when lifetimes are non-lexical
    match *input.next()? {
        Token::Number { value, .. } if clamping_mode.is_ok(context.parsing_mode, value) => {
            return Ok(Number {
                value: value.min(f32::MAX).max(f32::MIN),
                calc_clamping_mode: None,
            })
        }
        Token::Function(ref name) if name.eq_ignore_ascii_case("calc") => {}
        ref t => return Err(BasicParseError::UnexpectedToken(t.clone()).into())
    }

    let result = input.parse_nested_block(|i| {
        CalcNode::parse_number(context, i)
    })?;

    Ok(Number {
        value: result.min(f32::MAX).max(f32::MIN),
        calc_clamping_mode: Some(clamping_mode),
    })
}

#[derive(Clone, Copy, Debug, HasViewportPercentage, PartialEq)]
#[cfg_attr(feature = "servo", derive(HeapSizeOf, Deserialize, Serialize))]
/// An angle consisting of a value and a unit.
///
/// Computed Angle is essentially same as specified angle except calc
/// value serialization. Therefore we are using computed Angle enum
/// to hold the value and unit type.
pub struct Angle {
    value: computed::Angle,
    was_calc: bool,
}

impl ToCss for Angle {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
        if self.was_calc {
            dest.write_str("calc(")?;
        }
        self.value.to_css(dest)?;
        if self.was_calc {
            dest.write_str(")")?;
        }
        Ok(())
    }
}

impl ToComputedValue for Angle {
    type ComputedValue = computed::Angle;

    fn to_computed_value(&self, _context: &Context) -> Self::ComputedValue {
        self.value
    }

    fn from_computed_value(computed: &Self::ComputedValue) -> Self {
        Angle {
            value: *computed,
            was_calc: false,
        }
    }
}

impl Angle {
    /// Returns an angle with the given value in degrees.
    pub fn from_degrees(value: CSSFloat, was_calc: bool) -> Self {
        Angle { value: computed::Angle::Degree(value), was_calc: was_calc }
    }

    /// Returns an angle with the given value in gradians.
    pub fn from_gradians(value: CSSFloat, was_calc: bool) -> Self {
        Angle { value: computed::Angle::Gradian(value), was_calc: was_calc }
    }

    /// Returns an angle with the given value in turns.
    pub fn from_turns(value: CSSFloat, was_calc: bool) -> Self {
        Angle { value: computed::Angle::Turn(value), was_calc: was_calc }
    }

    /// Returns an angle with the given value in radians.
    pub fn from_radians(value: CSSFloat, was_calc: bool) -> Self {
        Angle { value: computed::Angle::Radian(value), was_calc: was_calc }
    }

    #[inline]
    #[allow(missing_docs)]
    pub fn radians(self) -> f32 {
        self.value.radians()
    }

    /// Returns an angle value that represents zero.
    pub fn zero() -> Self {
        Self::from_degrees(0.0, false)
    }

    /// Returns an `Angle` parsed from a `calc()` expression.
    pub fn from_calc(radians: CSSFloat) -> Self {
        Angle {
            value: computed::Angle::Radian(radians),
            was_calc: true,
        }
    }
}

impl Parse for Angle {
    /// Parses an angle according to CSS-VALUES § 6.1.
    fn parse<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i>> {
        // FIXME: remove clone() when lifetimes are non-lexical
        let token = input.next()?.clone();
        match token {
            Token::Dimension { value, ref unit, .. } => {
                Angle::parse_dimension(value, unit, /* from_calc = */ false)
            }
            Token::Function(ref name) if name.eq_ignore_ascii_case("calc") => {
                return input.parse_nested_block(|i| CalcNode::parse_angle(context, i))
            }
            _ => Err(())
        }.map_err(|()| BasicParseError::UnexpectedToken(token.clone()).into())
    }
}

impl Angle {
    /// Parse an `<angle>` value given a value and an unit.
    pub fn parse_dimension(
        value: CSSFloat,
        unit: &str,
        from_calc: bool)
        -> Result<Angle, ()>
    {
        let angle = match_ignore_ascii_case! { unit,
            "deg" => Angle::from_degrees(value, from_calc),
            "grad" => Angle::from_gradians(value, from_calc),
            "turn" => Angle::from_turns(value, from_calc),
            "rad" => Angle::from_radians(value, from_calc),
             _ => return Err(())
        };
        Ok(angle)
    }
    /// Parse an angle, including unitless 0 degree.
    ///
    /// Note that numbers without any AngleUnit, including unitless 0 angle,
    /// should be invalid. However, some properties still accept unitless 0
    /// angle and stores it as '0deg'.
    ///
    /// We can remove this and get back to the unified version Angle::parse once
    /// https://github.com/w3c/csswg-drafts/issues/1162 is resolved.
    pub fn parse_with_unitless<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>)
                                       -> Result<Self, ParseError<'i>> {
        // FIXME: remove clone() when lifetimes are non-lexical
        let token = input.next()?.clone();
        match token {
            Token::Dimension { value, ref unit, .. } => {
                Angle::parse_dimension(value, unit, /* from_calc = */ false)
            }
            Token::Number { value, .. } if value == 0. => Ok(Angle::zero()),
            Token::Function(ref name) if name.eq_ignore_ascii_case("calc") => {
                return input.parse_nested_block(|i| CalcNode::parse_angle(context, i))
            }
            _ => Err(())
        }.map_err(|()| BasicParseError::UnexpectedToken(token.clone()).into())
    }
}

// The integer values here correspond to the border conflict resolution rules in CSS 2.1 §
// 17.6.2.1. Higher values override lower values.
define_numbered_css_keyword_enum! { BorderStyle:
    "none" => none = -1,
    "solid" => solid = 6,
    "double" => double = 7,
    "dotted" => dotted = 4,
    "dashed" => dashed = 5,
    "hidden" => hidden = -2,
    "groove" => groove = 1,
    "ridge" => ridge = 3,
    "inset" => inset = 0,
    "outset" => outset = 2,
}

no_viewport_percentage!(BorderStyle);

impl BorderStyle {
    /// Whether this border style is either none or hidden.
    pub fn none_or_hidden(&self) -> bool {
        matches!(*self, BorderStyle::none | BorderStyle::hidden)
    }
}

/// Time unit.
#[derive(Clone, Copy, Debug, HasViewportPercentage, PartialEq, Eq)]
#[cfg_attr(feature = "servo", derive(HeapSizeOf))]
pub enum TimeUnit {
    /// `s`
    Second,
    /// `ms`
    Millisecond,
}

/// A time in seconds according to CSS-VALUES § 6.2.
#[derive(Clone, Copy, Debug, HasViewportPercentage, PartialEq)]
#[cfg_attr(feature = "servo", derive(HeapSizeOf))]
pub struct Time {
    seconds: CSSFloat,
    unit: TimeUnit,
    was_calc: bool,
}

impl Time {
    /// Return a `<time>` value that represents `seconds` seconds.
    pub fn from_seconds(seconds: CSSFloat) -> Self {
        Time {
            seconds: seconds,
            unit: TimeUnit::Second,
            was_calc: false,
        }
    }

    /// Returns a time that represents a duration of zero.
    pub fn zero() -> Self {
        Self::from_seconds(0.0)
    }

    /// Returns the time in fractional seconds.
    pub fn seconds(self) -> CSSFloat {
        self.seconds
    }

    /// Parses a time according to CSS-VALUES § 6.2.
    pub fn parse_dimension(
        value: CSSFloat,
        unit: &str,
        from_calc: bool)
        -> Result<Time, ()>
    {
        let (seconds, unit) = match_ignore_ascii_case! { unit,
            "s" => (value, TimeUnit::Second),
            "ms" => (value / 1000.0, TimeUnit::Millisecond),
            _ => return Err(())
        };

        Ok(Time {
            seconds: seconds,
            unit: unit,
            was_calc: from_calc,
        })
    }

    /// Returns a `Time` value from a CSS `calc()` expression.
    pub fn from_calc(seconds: CSSFloat) -> Self {
        Time {
            seconds: seconds,
            unit: TimeUnit::Second,
            was_calc: true,
        }
    }

    fn parse_with_clamping_mode<'i, 't>(context: &ParserContext,
                                        input: &mut Parser<'i, 't>,
                                        clamping_mode: AllowedNumericType)
                                        -> Result<Self, ParseError<'i>> {
        use style_traits::PARSING_MODE_DEFAULT;

        // FIXME: remove early returns when lifetimes are non-lexical
        match input.next() {
            // Note that we generally pass ParserContext to is_ok() to check
            // that the ParserMode of the ParserContext allows all numeric
            // values for SMIL regardless of clamping_mode, but in this Time
            // value case, the value does not animate for SMIL at all, so we use
            // PARSING_MODE_DEFAULT directly.
            Ok(&Token::Dimension { value, ref unit, .. }) if clamping_mode.is_ok(PARSING_MODE_DEFAULT, value) => {
                return Time::parse_dimension(value, unit, /* from_calc = */ false)
                    .map_err(|()| StyleParseError::UnspecifiedError.into())
            }
            Ok(&Token::Function(ref name)) if name.eq_ignore_ascii_case("calc") => {}
            Ok(t) => return Err(BasicParseError::UnexpectedToken(t.clone()).into()),
            Err(e) => return Err(e.into())
        }
        match input.parse_nested_block(|i| CalcNode::parse_time(context, i)) {
            Ok(time) if clamping_mode.is_ok(PARSING_MODE_DEFAULT, time.seconds) => Ok(time),
            _ => Err(StyleParseError::UnspecifiedError.into()),
        }
    }

    /// Parse <time> that values are non-negative.
    pub fn parse_non_negative<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>)
                                      -> Result<Self, ParseError<'i>> {
        Self::parse_with_clamping_mode(context, input, AllowedNumericType::NonNegative)
    }
}

impl ToComputedValue for Time {
    type ComputedValue = computed::Time;

    fn to_computed_value(&self, _context: &Context) -> Self::ComputedValue {
        computed::Time::from_seconds(self.seconds())
    }

    fn from_computed_value(computed: &Self::ComputedValue) -> Self {
        Time {
            seconds: computed.seconds(),
            unit: TimeUnit::Second,
            was_calc: false,
        }
    }
}

impl Parse for Time {
    fn parse<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i>> {
        Self::parse_with_clamping_mode(context, input, AllowedNumericType::All)
    }
}

impl ToCss for Time {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
        if self.was_calc {
            dest.write_str("calc(")?;
        }
        match self.unit {
            TimeUnit::Second => {
                self.seconds.to_css(dest)?;
                dest.write_str("s")?;
            }
            TimeUnit::Millisecond => {
                (self.seconds * 1000.).to_css(dest)?;
                dest.write_str("ms")?;
            }
        }
        if self.was_calc {
            dest.write_str(")")?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "servo", derive(HeapSizeOf))]
#[allow(missing_docs)]
pub struct Number {
    /// The numeric value itself.
    value: CSSFloat,
    /// If this number came from a calc() expression, this tells how clamping
    /// should be done on the value.
    calc_clamping_mode: Option<AllowedNumericType>,
}

no_viewport_percentage!(Number);

impl Parse for Number {
    fn parse<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i>> {
        parse_number(context, input)
    }
}

impl Number {
    /// Returns a new number with the value `val`.
    pub fn new(val: CSSFloat) -> Self {
        Number {
            value: val,
            calc_clamping_mode: None,
        }
    }

    /// Returns the numeric value, clamped if needed.
    pub fn get(&self) -> f32 {
        self.calc_clamping_mode.map_or(self.value, |mode| mode.clamp(self.value))
    }

    #[allow(missing_docs)]
    pub fn parse_non_negative<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>)
                                      -> Result<Number, ParseError<'i>> {
        parse_number_with_clamping_mode(context, input, AllowedNumericType::NonNegative)
    }

    #[allow(missing_docs)]
    pub fn parse_at_least_one<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>)
                                      -> Result<Number, ParseError<'i>> {
        parse_number_with_clamping_mode(context, input, AllowedNumericType::AtLeastOne)
    }
}

impl ToComputedValue for Number {
    type ComputedValue = CSSFloat;

    #[inline]
    fn to_computed_value(&self, _: &Context) -> CSSFloat { self.get() }

    #[inline]
    fn from_computed_value(computed: &CSSFloat) -> Self {
        Number {
            value: *computed,
            calc_clamping_mode: None,
        }
    }
}

impl ToCss for Number {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
        where W: fmt::Write,
    {
        if self.calc_clamping_mode.is_some() {
            dest.write_str("calc(")?;
        }
        self.value.to_css(dest)?;
        if self.calc_clamping_mode.is_some() {
            dest.write_str(")")?;
        }
        Ok(())
    }
}

/// A Number which is >= 0.0.
pub type NonNegativeNumber = NonNegative<Number>;

impl Parse for NonNegativeNumber {
    fn parse<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i>> {
        parse_number_with_clamping_mode(context, input, AllowedNumericType::NonNegative)
            .map(NonNegative::<Number>)
    }
}

impl NonNegativeNumber {
    /// Returns a new non-negative number with the value `val`.
    pub fn new(val: CSSFloat) -> Self {
        NonNegative::<Number>(Number::new(val.max(0.)))
    }
}

/// A Number which is >= 1.0.
pub type GreaterThanOrEqualToOneNumber = GreaterThanOrEqualToOne<Number>;

impl Parse for GreaterThanOrEqualToOneNumber {
    fn parse<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i>> {
        parse_number_with_clamping_mode(context, input, AllowedNumericType::AtLeastOne)
            .map(GreaterThanOrEqualToOne::<Number>)
    }
}

/// <number> | <percentage>
///
/// Accepts only non-negative numbers.
///
/// FIXME(emilio): Should probably use Either.
#[allow(missing_docs)]
#[cfg_attr(feature = "servo", derive(HeapSizeOf))]
#[derive(Clone, Copy, Debug, PartialEq, ToCss)]
pub enum NumberOrPercentage {
    Percentage(Percentage),
    Number(Number),
}

no_viewport_percentage!(NumberOrPercentage);

impl NumberOrPercentage {
    fn parse_with_clamping_mode<'i, 't>(
        context: &ParserContext,
        input: &mut Parser<'i, 't>,
        type_: AllowedNumericType
    ) -> Result<Self, ParseError<'i>> {
        if let Ok(per) = input.try(|i| Percentage::parse_with_clamping_mode(context, i, type_)) {
            return Ok(NumberOrPercentage::Percentage(per));
        }

        parse_number_with_clamping_mode(context, input, type_).map(NumberOrPercentage::Number)
    }

    /// Parse a non-negative number or percentage.
    pub fn parse_non_negative<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>)
                                      -> Result<Self, ParseError<'i>> {
        Self::parse_with_clamping_mode(context, input, AllowedNumericType::NonNegative)
    }
}

impl Parse for NumberOrPercentage {
    fn parse<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i>> {
        Self::parse_with_clamping_mode(context, input, AllowedNumericType::All)
    }
}

#[allow(missing_docs)]
#[cfg_attr(feature = "servo", derive(HeapSizeOf))]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, ToCss)]
pub struct Opacity(Number);

no_viewport_percentage!(Opacity);

impl Parse for Opacity {
    fn parse<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i>> {
        parse_number(context, input).map(Opacity)
    }
}

impl ToComputedValue for Opacity {
    type ComputedValue = CSSFloat;

    #[inline]
    fn to_computed_value(&self, context: &Context) -> CSSFloat {
        let value = self.0.to_computed_value(context);
        if context.for_smil_animation {
            // SMIL expects to be able to interpolate between out-of-range
            // opacity values.
            value
        } else {
            value.min(1.0).max(0.0)
        }
    }

    #[inline]
    fn from_computed_value(computed: &CSSFloat) -> Self {
        Opacity(Number::from_computed_value(computed))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "servo", derive(HeapSizeOf))]
#[allow(missing_docs)]
pub struct Integer {
    value: CSSInteger,
    was_calc: bool,
}

impl Integer {
    /// Trivially constructs a new `Integer` value.
    pub fn new(val: CSSInteger) -> Self {
        Integer {
            value: val,
            was_calc: false,
        }
    }

    /// Returns the integer value associated with this value.
    pub fn value(&self) -> CSSInteger {
        self.value
    }

    /// Trivially constructs a new integer value from a `calc()` expression.
    fn from_calc(val: CSSInteger) -> Self {
        Integer {
            value: val,
            was_calc: true,
        }
    }
}

no_viewport_percentage!(Integer);

impl Parse for Integer {
    fn parse<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i>> {
        parse_integer(context, input)
    }
}

impl Integer {
    #[allow(missing_docs)]
    pub fn parse_with_minimum<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>, min: i32)
                                      -> Result<Integer, ParseError<'i>> {
        match parse_integer(context, input) {
            Ok(value) if value.value() >= min => Ok(value),
            Ok(_value) => Err(StyleParseError::UnspecifiedError.into()),
            Err(e) => Err(e),
        }
    }

    #[allow(missing_docs)]
    pub fn parse_non_negative<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>)
                                      -> Result<Integer, ParseError<'i>> {
        Integer::parse_with_minimum(context, input, 0)
    }

    #[allow(missing_docs)]
    pub fn parse_positive<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>)
                                  -> Result<Integer, ParseError<'i>> {
        Integer::parse_with_minimum(context, input, 1)
    }
}

impl ToComputedValue for Integer {
    type ComputedValue = i32;

    #[inline]
    fn to_computed_value(&self, _: &Context) -> i32 { self.value }

    #[inline]
    fn from_computed_value(computed: &i32) -> Self {
        Integer::new(*computed)
    }
}

impl ToCss for Integer {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
        where W: fmt::Write,
    {
        if self.was_calc {
            dest.write_str("calc(")?;
        }
        write!(dest, "{}", self.value)?;
        if self.was_calc {
            dest.write_str(")")?;
        }
        Ok(())
    }
}

/// <integer> | auto
pub type IntegerOrAuto = Either<Integer, Auto>;

impl IntegerOrAuto {
    #[allow(missing_docs)]
    pub fn parse_positive<'i, 't>(context: &ParserContext,
                                  input: &mut Parser<'i, 't>)
                                  -> Result<IntegerOrAuto, ParseError<'i>> {
        match IntegerOrAuto::parse(context, input) {
            Ok(Either::First(integer)) if integer.value() <= 0 => Err(StyleParseError::UnspecifiedError.into()),
            result => result,
        }
    }
}

/// A wrapper of Integer, with value >= 1.
pub type PositiveInteger = GreaterThanOrEqualToOne<Integer>;

impl Parse for PositiveInteger {
    #[inline]
    fn parse<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i>> {
        Integer::parse_positive(context, input).map(GreaterThanOrEqualToOne::<Integer>)
    }
}

/// PositiveInteger | auto
pub type PositiveIntegerOrAuto = Either<PositiveInteger, Auto>;

#[allow(missing_docs)]
pub type UrlOrNone = Either<SpecifiedUrl, None_>;

/// The specified value of a grid `<track-breadth>`
pub type TrackBreadth = GenericTrackBreadth<LengthOrPercentage>;

/// The specified value of a grid `<track-size>`
pub type TrackSize = GenericTrackSize<LengthOrPercentage>;

/// The specified value of a grid `<track-list>`
/// (could also be `<auto-track-list>` or `<explicit-track-list>`)
pub type TrackList = GenericTrackList<LengthOrPercentage>;

/// `<grid-template-rows> | <grid-template-columns>`
pub type GridTemplateComponent = GenericGridTemplateComponent<LengthOrPercentage>;

/// <length> | <percentage> | <number>
pub type LengthOrPercentageOrNumber = Either<Number, LengthOrPercentage>;

/// NonNegativeLengthOrPercentage | NonNegativeNumber
pub type NonNegativeLengthOrPercentageOrNumber = Either<NonNegativeNumber, NonNegativeLengthOrPercentage>;

#[derive(Clone, Debug, HasViewportPercentage, PartialEq)]
#[cfg_attr(feature = "servo", derive(HeapSizeOf))]
/// rect(<top>, <left>, <bottom>, <right>) used by clip and image-region
pub struct ClipRect {
    /// <top> (<length> | <auto>)
    pub top: Option<Length>,
    /// <right> (<length> | <auto>)
    pub right: Option<Length>,
    /// <bottom> (<length> | <auto>)
    pub bottom: Option<Length>,
    /// <left> (<length> | <auto>)
    pub left: Option<Length>,
}


impl ToCss for ClipRect {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
        dest.write_str("rect(")?;

        if let Some(ref top) = self.top {
            top.to_css(dest)?;
            dest.write_str(", ")?;
        } else {
            dest.write_str("auto, ")?;
        }

        if let Some(ref right) = self.right {
            right.to_css(dest)?;
            dest.write_str(", ")?;
        } else {
            dest.write_str("auto, ")?;
        }

        if let Some(ref bottom) = self.bottom {
            bottom.to_css(dest)?;
            dest.write_str(", ")?;
        } else {
            dest.write_str("auto, ")?;
        }

        if let Some(ref left) = self.left {
            left.to_css(dest)?;
        } else {
            dest.write_str("auto")?;
        }

        dest.write_str(")")?;
        Ok(())
    }
}

impl ToComputedValue for ClipRect {
    type ComputedValue = super::computed::ClipRect;

    #[inline]
    fn to_computed_value(&self, context: &Context) -> super::computed::ClipRect {
        super::computed::ClipRect {
            top: self.top.as_ref().map(|top| top.to_computed_value(context)),
            right: self.right.as_ref().map(|right| right.to_computed_value(context)),
            bottom: self.bottom.as_ref().map(|bottom| bottom.to_computed_value(context)),
            left: self.left.as_ref().map(|left| left.to_computed_value(context)),
        }
    }

    #[inline]
    fn from_computed_value(computed: &super::computed::ClipRect) -> Self {
        ClipRect {
            top: computed.top.map(|top| ToComputedValue::from_computed_value(&top)),
            right: computed.right.map(|right| ToComputedValue::from_computed_value(&right)),
            bottom: computed.bottom.map(|bottom| ToComputedValue::from_computed_value(&bottom)),
            left: computed.left.map(|left| ToComputedValue::from_computed_value(&left)),
        }
    }
}

impl Parse for ClipRect {
    fn parse<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>) -> Result<Self, ParseError<'i>> {
        Self::parse_quirky(context, input, AllowQuirks::No)
    }
}

impl ClipRect {
    /// Parses a rect(<top>, <left>, <bottom>, <right>), allowing quirks.
    pub fn parse_quirky<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>,
                                allow_quirks: AllowQuirks) -> Result<Self, ParseError<'i>> {
        use values::specified::Length;

        fn parse_argument<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>,
                                  allow_quirks: AllowQuirks) -> Result<Option<Length>, ParseError<'i>> {
            if input.try(|input| input.expect_ident_matching("auto")).is_ok() {
                Ok(None)
            } else {
                Length::parse_quirky(context, input, allow_quirks).map(Some)
            }
        }

        input.expect_function_matching("rect")?;

        input.parse_nested_block(|input| {
            let top = parse_argument(context, input, allow_quirks)?;
            let right;
            let bottom;
            let left;

            if input.try(|input| input.expect_comma()).is_ok() {
                right = parse_argument(context, input, allow_quirks)?;
                input.expect_comma()?;
                bottom = parse_argument(context, input, allow_quirks)?;
                input.expect_comma()?;
                left = parse_argument(context, input, allow_quirks)?;
            } else {
                right = parse_argument(context, input, allow_quirks)?;
                bottom = parse_argument(context, input, allow_quirks)?;
                left = parse_argument(context, input, allow_quirks)?;
            }
            Ok(ClipRect {
                top: top,
                right: right,
                bottom: bottom,
                left: left,
            })
        })
    }
}

/// rect(...) | auto
pub type ClipRectOrAuto = Either<ClipRect, Auto>;

impl ClipRectOrAuto {
    /// Parses a ClipRect or Auto, allowing quirks.
    pub fn parse_quirky<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>,
                                allow_quirks: AllowQuirks) -> Result<Self, ParseError<'i>> {
        if let Ok(v) = input.try(|i| ClipRect::parse_quirky(context, i, allow_quirks)) {
            Ok(Either::First(v))
        } else {
            Auto::parse(context, input).map(Either::Second)
        }
    }
}

/// <color> | auto
pub type ColorOrAuto = Either<Color, Auto>;

/// Whether quirks are allowed in this context.
#[derive(Clone, Copy, PartialEq)]
pub enum AllowQuirks {
    /// Quirks are allowed.
    Yes,
    /// Quirks are not allowed.
    No,
}

impl AllowQuirks {
    /// Returns `true` if quirks are allowed in this context.
    pub fn allowed(self, quirks_mode: QuirksMode) -> bool {
        self == AllowQuirks::Yes && quirks_mode == QuirksMode::Quirks
    }
}

#[cfg(feature = "gecko")]
/// A namespace ID
pub type NamespaceId = i32;


#[cfg(feature = "servo")]
/// A namespace ID (used by gecko only)
pub type NamespaceId = ();

/// An attr(...) rule
///
/// `[namespace? `|`]? ident`
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "servo", derive(HeapSizeOf))]
pub struct Attr {
    /// Optional namespace
    pub namespace: Option<(Namespace, NamespaceId)>,
    /// Attribute name
    pub attribute: String,
}

impl Parse for Attr {
    fn parse<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>) -> Result<Attr, ParseError<'i>> {
        input.expect_function_matching("attr")?;
        input.parse_nested_block(|i| Attr::parse_function(context, i))
    }
}

#[cfg(feature = "gecko")]
/// Get the namespace id from the namespace map
fn get_id_for_namespace(namespace: &Namespace, context: &ParserContext) -> Result<NamespaceId, ()> {
    let namespaces_map = match context.namespaces {
        Some(map) => map,
        None => {
            // If we don't have a namespace map (e.g. in inline styles)
            // we can't parse namespaces
            return Err(());
        }
    };

    match namespaces_map.prefixes.get(&namespace.0) {
        Some(entry) => Ok(entry.1),
        None => Err(()),
    }
}

#[cfg(feature = "servo")]
/// Get the namespace id from the namespace map
fn get_id_for_namespace(_: &Namespace, _: &ParserContext) -> Result<NamespaceId, ()> {
    Ok(())
}

impl Attr {
    /// Parse contents of attr() assuming we have already parsed `attr` and are
    /// within a parse_nested_block()
    pub fn parse_function<'i, 't>(context: &ParserContext, input: &mut Parser<'i, 't>)
                                  -> Result<Attr, ParseError<'i>> {
        // Syntax is `[namespace? `|`]? ident`
        // no spaces allowed
        let first = input.try(|i| i.expect_ident_cloned()).ok();
        if let Ok(token) = input.try(|i| i.next_including_whitespace().map(|t| t.clone())) {
            match token {
                Token::Delim('|') => {}
                ref t => return Err(BasicParseError::UnexpectedToken(t.clone()).into()),
            }
            // must be followed by an ident
            let second_token = match *input.next_including_whitespace()? {
                Token::Ident(ref second) => second,
                ref t => return Err(BasicParseError::UnexpectedToken(t.clone()).into()),
            };

            let ns_with_id = if let Some(ns) = first {
                let ns = Namespace::from(ns.as_ref());
                let id: Result<_, ParseError> =
                    get_id_for_namespace(&ns, context)
                    .map_err(|()| StyleParseError::UnspecifiedError.into());
                Some((ns, id?))
            } else {
                None
            };
            return Ok(Attr {
                namespace: ns_with_id,
                attribute: second_token.as_ref().to_owned(),
            })
        }

        if let Some(first) = first {
            Ok(Attr {
                namespace: None,
                attribute: first.as_ref().to_owned(),
            })
        } else {
            Err(StyleParseError::UnspecifiedError.into())
        }
    }
}

impl ToCss for Attr {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result where W: fmt::Write {
        dest.write_str("attr(")?;
        if let Some(ref ns) = self.namespace {
            serialize_identifier(&ns.0.to_string(), dest)?;
            dest.write_str("|")?;
        }
        serialize_identifier(&self.attribute, dest)?;
        dest.write_str(")")
    }
}

impl ComputedValueAsSpecified for Attr {}

/// A percentage value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "servo", derive(HeapSizeOf))]
pub struct Percentage {
    /// The percentage value as a float.
    ///
    /// [0 .. 100%] maps to [0.0 .. 1.0]
    value: CSSFloat,
    /// If this percentage came from a calc() expression, this tells how
    /// clamping should be done on the value.
    calc_clamping_mode: Option<AllowedNumericType>,
}

no_viewport_percentage!(Percentage);

impl ToCss for Percentage {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
        where W: fmt::Write,
    {
        if self.calc_clamping_mode.is_some() {
            dest.write_str("calc(")?;
        }

        write!(dest, "{}%", self.value * 100.)?;

        if self.calc_clamping_mode.is_some() {
            dest.write_str(")")?;
        }
        Ok(())
    }
}

impl Percentage {
    /// Create a percentage from a numeric value.
    pub fn new(value: CSSFloat) -> Self {
        Self {
            value,
            calc_clamping_mode: None,
        }
    }

    /// Get the underlying value for this float.
    pub fn get(&self) -> CSSFloat {
        self.calc_clamping_mode.map_or(self.value, |mode| mode.clamp(self.value))
    }

    /// Reverse this percentage, preserving calc-ness.
    ///
    /// For example: If it was 20%, convert it into 80%.
    pub fn reverse(&mut self) {
        let new_value = 1. - self.value;
        self.value = new_value;
    }


    /// Parse a specific kind of percentage.
    pub fn parse_with_clamping_mode<'i, 't>(
        context: &ParserContext,
        input: &mut Parser<'i, 't>,
        num_context: AllowedNumericType,
    ) -> Result<Self, ParseError<'i>> {
        // FIXME: remove early returns when lifetimes are non-lexical
        match *input.next()? {
            Token::Percentage { unit_value, .. } if num_context.is_ok(context.parsing_mode, unit_value) => {
                return Ok(Percentage::new(unit_value))
            }
            Token::Function(ref name) if name.eq_ignore_ascii_case("calc") => {}
            ref t => return Err(BasicParseError::UnexpectedToken(t.clone()).into())
        }

        let result = input.parse_nested_block(|i| {
            CalcNode::parse_percentage(context, i)
        })?;

        // TODO(emilio): -moz-image-rect is the only thing that uses
        // the clamping mode... I guess we could disallow it...
        Ok(Percentage {
            value: result,
            calc_clamping_mode: Some(num_context),
        })
    }

    /// Parses a percentage token, but rejects it if it's negative.
    pub fn parse_non_negative<'i, 't>(context: &ParserContext,
                                      input: &mut Parser<'i, 't>)
                                      -> Result<Self, ParseError<'i>> {
        Self::parse_with_clamping_mode(context, input, AllowedNumericType::NonNegative)
    }

    /// 0%
    #[inline]
    pub fn zero() -> Self {
        Percentage {
            value: 0.,
            calc_clamping_mode: None,
        }
    }

    /// 100%
    #[inline]
    pub fn hundred() -> Self {
        Percentage {
            value: 1.,
            calc_clamping_mode: None,
        }
    }
}

impl Parse for Percentage {
    #[inline]
    fn parse<'i, 't>(
        context: &ParserContext,
        input: &mut Parser<'i, 't>
    ) -> Result<Self, ParseError<'i>> {
        Self::parse_with_clamping_mode(context, input, AllowedNumericType::All)
    }
}
