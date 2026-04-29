use std::cmp::Ordering;
use std::path::Path;

use super::parse_variable_name_token;
use crate::command_text::interpolate_argv;
use crate::runtime::{BumpKind, RuntimeEnv};
use crate::version::{compare_semver, is_prerelease};
use crate::{CjError, CjResult};

pub(super) fn evaluate_condition(
    base_dir: &Path,
    name: &str,
    args: &str,
    effective_env: &RuntimeEnv,
) -> CjResult<bool> {
    match name {
        "if" => evaluate_if_condition(args, effective_env),
        "if-not" => evaluate_if_condition(args, effective_env).map(|result| !result),
        "if-in" | "if-not-in" => evaluate_if_in_condition(args, effective_env).map(|result| {
            if name == "if-in" {
                result
            } else {
                !result
            }
        }),
        "if-exists" | "if-not-exists" => {
            let argv = interpolate_argv(args, &effective_env.vars)?;
            if argv.len() != 1 {
                return Err(CjError::new(format!("@{name} expects exactly one path")));
            }
            let path = base_dir.join(&argv[0]);
            Ok(if name == "if-exists" {
                path.exists()
            } else {
                !path.exists()
            })
        }
        "if-set" | "if-not-set" => {
            let argv = interpolate_argv(args, &effective_env.vars)?;
            if argv.len() != 1 {
                return Err(CjError::new(format!(
                    "@{name} expects exactly one variable name"
                )));
            }
            let variable = parse_variable_name_token(&argv[0])?;
            let exists = effective_env.vars.contains_key(&variable);
            Ok(if name == "if-set" { exists } else { !exists })
        }
        "if-version" | "if-not-version" => {
            evaluate_version_condition(args, effective_env).map(|result| {
                if name == "if-version" {
                    result
                } else {
                    !result
                }
            })
        }
        "if-bumped" | "if-not-bumped" => evaluate_bumped_condition(name, args, effective_env),
        "if-patch" | "if-minor" | "if-major" | "if-pre" | "if-release" | "if-not-patch"
        | "if-not-minor" | "if-not-major" | "if-not-pre" | "if-not-release" => {
            evaluate_bump_kind_condition(name, args, effective_env)
        }
        _ => unreachable!("condition directive checked by caller"),
    }
}

pub(crate) fn if_condition_error() -> &'static str {
    "@if expects a value, '<left> == <right>', or '<left> != <right>'"
}

fn evaluate_if_condition(args: &str, effective_env: &RuntimeEnv) -> CjResult<bool> {
    let argv = interpolate_argv(args, &effective_env.vars)?;
    match argv.as_slice() {
        [value] => Ok(is_truthy(value)),
        [left, op, right] if op == "==" => Ok(left == right),
        [left, op, right] if op == "!=" => Ok(left != right),
        _ => Err(CjError::new(if_condition_error())),
    }
}

pub(crate) fn if_in_condition_error() -> &'static str {
    "@if-in expects '<needle> <candidate>...'"
}

fn evaluate_if_in_condition(args: &str, effective_env: &RuntimeEnv) -> CjResult<bool> {
    let argv = interpolate_argv(args, &effective_env.vars)?;
    match argv.as_slice() {
        [needle, candidates @ ..] if !candidates.is_empty() => Ok(candidates.contains(needle)),
        _ => Err(CjError::new(if_in_condition_error())),
    }
}

fn evaluate_version_condition(args: &str, effective_env: &RuntimeEnv) -> CjResult<bool> {
    let argv = interpolate_argv(args, &effective_env.vars)?;
    match argv.as_slice() {
        [name, state] if state == "prerelease" || state == "pre" => {
            let value = version_value(effective_env, name)?;
            is_prerelease(&value, 0)
        }
        [name, state] if state == "release" => {
            let value = version_value(effective_env, name)?;
            Ok(!is_prerelease(&value, 0)?)
        }
        [name, op, right] if matches!(op.as_str(), "==" | "!=" | "<" | "<=" | ">" | ">=") => {
            let left = version_value(effective_env, name)?;
            let ordering = compare_semver(&left, right, 0)?;
            Ok(match op.as_str() {
                "==" => ordering == Ordering::Equal,
                "!=" => ordering != Ordering::Equal,
                "<" => ordering == Ordering::Less,
                "<=" => ordering != Ordering::Greater,
                ">" => ordering == Ordering::Greater,
                ">=" => ordering != Ordering::Less,
                _ => unreachable!(),
            })
        }
        _ => Err(CjError::new(
            "@if-version expects '<name> <op> <version>', '<name> prerelease', or '<name> release'",
        )),
    }
}

fn evaluate_bumped_condition(
    directive: &str,
    args: &str,
    effective_env: &RuntimeEnv,
) -> CjResult<bool> {
    let expect_bumped = directive == "if-bumped";
    let argv = interpolate_argv(args, &effective_env.vars)?;
    let result = match argv.as_slice() {
        [] => !effective_env.bumped_versions.is_empty(),
        [name] => effective_env.bumped_versions.contains_key(name),
        _ => {
            return Err(CjError::new(format!(
                "@{directive} expects no arguments or '<name>'",
            )))
        }
    };
    Ok(if expect_bumped { result } else { !result })
}

fn evaluate_bump_kind_condition(
    directive: &str,
    args: &str,
    effective_env: &RuntimeEnv,
) -> CjResult<bool> {
    let argv = interpolate_argv(args, &effective_env.vars)?;
    let [name] = argv.as_slice() else {
        return Err(CjError::new(format!(
            "@{directive} expects exactly one name"
        )));
    };
    let kind_name = directive
        .strip_prefix("if-")
        .expect("bump kind condition must start with if-");
    let (kind_name, expect_match) = kind_name
        .strip_prefix("not-")
        .map_or((kind_name, true), |positive| (positive, false));
    let expected = BumpKind::parse(kind_name).expect("bump kind condition must use a bump kind");
    let result = effective_env
        .bumped_versions
        .get(name)
        .is_some_and(|actual| *actual == expected);
    Ok(if expect_match { result } else { !result })
}

fn version_value(effective_env: &RuntimeEnv, name: &str) -> CjResult<String> {
    let key = crate::version::version_env_key(name)
        .map_err(|err| CjError::new(format!("invalid version name '{name}': {err}")))?;
    effective_env
        .vars
        .get(&key)
        .cloned()
        .ok_or_else(|| CjError::new(format!("unknown version '{name}'")))
}

pub(super) fn is_truthy(value: &str) -> bool {
    !(value.is_empty() || value == "0" || value.eq_ignore_ascii_case("false"))
}
