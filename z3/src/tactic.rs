use std::ffi::{CStr, CString};
use std::fmt;
use std::result::Result;
use std::str::Utf8Error;

use z3_sys::*;
use ApplyResult;
use Context;
use Goal;
use Params;
use Probe;
use Tactic;

impl<'ctx> ApplyResult<'ctx> {
    pub fn list_subgoals(self) -> impl Iterator<Item = Goal<'ctx>> {
        let num_subgoals =
            unsafe { Z3_apply_result_get_num_subgoals(self.ctx.z3_ctx, self.z3_apply_result) };
        (0..num_subgoals).into_iter().map(move |i| unsafe {
            let sg = Z3_apply_result_get_subgoal(self.ctx.z3_ctx, self.z3_apply_result, i);
            Z3_goal_inc_ref(self.ctx.z3_ctx, sg);
            Goal::new_from_z3_type(self.ctx, sg, true, true, true)
        })
    }
}

impl<'ctx> Drop for ApplyResult<'ctx> {
    fn drop(&mut self) {
        unsafe {
            Z3_apply_result_dec_ref(self.ctx.z3_ctx, self.z3_apply_result);
        }
    }
}

impl<'ctx> Tactic<'ctx> {
    pub fn list_all(
        ctx: &'ctx Context,
    ) -> impl Iterator<Item = std::result::Result<&'ctx str, Utf8Error>> {
        let p = unsafe { Z3_get_num_tactics(ctx.z3_ctx) };
        (0..p).into_iter().map(move |n| {
            let t = unsafe { Z3_get_tactic_name(ctx.z3_ctx, n) };
            unsafe { CStr::from_ptr(t) }.to_str()
        })
    }

    pub(crate) fn new_from_z3(ctx: &'ctx Context, z3_tactic: Z3_tactic) -> Tactic<'ctx> {
        Tactic { ctx, z3_tactic }
    }

    pub fn new(ctx: &'ctx Context, name: &str) -> Tactic<'ctx> {
        let tactic_name = CString::new(name).unwrap();
        Tactic {
            ctx,
            z3_tactic: unsafe {
                let t = Z3_mk_tactic(ctx.z3_ctx, tactic_name.as_ptr());
                Z3_tactic_inc_ref(ctx.z3_ctx, t);
                t
            },
        }
    }

    /// Return a tactic that just return the given goal.
    pub fn create_skip(ctx: &'ctx Context) -> Tactic<'ctx> {
        Tactic::new_from_z3(ctx, unsafe {
            let t = Z3_tactic_skip(ctx.z3_ctx);
            Z3_tactic_inc_ref(ctx.z3_ctx, t);
            t
        })
    }

    /// Return a tactic that always fails.
    pub fn create_fail(ctx: &'ctx Context) -> Tactic<'ctx> {
        Tactic::new_from_z3(ctx, unsafe {
            let t = Z3_tactic_fail(ctx.z3_ctx);
            Z3_tactic_inc_ref(ctx.z3_ctx, t);
            t
        })
    }

    /// Return a tactic that keeps applying `t` until the goal is not modified anymore or the maximum
    /// number of iterations `max` is reached.
    pub fn repeat(ctx: &'ctx Context, t: &Tactic<'ctx>, max: u32) -> Tactic<'ctx> {
        Tactic {
            ctx,
            z3_tactic: unsafe {
                let t = Z3_tactic_repeat(ctx.z3_ctx, t.z3_tactic, max);
                Z3_tactic_inc_ref(ctx.z3_ctx, t);
                t
            },
        }
    }

    /// Return a tactic that applies the current tactic to a given goal and
    /// the `then_tactic` to every subgoal produced by the original tactic.
    pub fn and_then(&self, then_tactic: &Tactic<'ctx>) -> Tactic<'ctx> {
        unsafe {
            let t = Z3_tactic_and_then(self.ctx.z3_ctx, self.z3_tactic, then_tactic.z3_tactic);
            Z3_tactic_inc_ref(self.ctx.z3_ctx, t);
            Tactic {
                ctx: self.ctx,
                z3_tactic: t,
            }
        }
    }

    /// Return a tactic that current tactic to a given goal,
    /// if it fails then returns the result of `else_tactic` applied to the given goal.
    pub fn or_else(&self, else_tactic: &Tactic<'ctx>) -> Tactic<'ctx> {
        unsafe {
            let t = Z3_tactic_or_else(self.ctx.z3_ctx, self.z3_tactic, else_tactic.z3_tactic);
            Z3_tactic_inc_ref(self.ctx.z3_ctx, t);
            Tactic {
                ctx: self.ctx,
                z3_tactic: t,
            }
        }
    }

    /// Return a tactic that applies self to a given goal if the probe `p` evaluates to true,
    /// and `t` if `p` evaluates to false.
    pub fn probe_or_else(&self, p: &Probe<'ctx>, t: &Tactic<'ctx>) -> Tactic<'ctx> {
        unsafe {
            let t = Z3_tactic_cond(self.ctx.z3_ctx, p.z3_probe, self.z3_tactic, t.z3_tactic);
            Z3_tactic_inc_ref(self.ctx.z3_ctx, t);
            Tactic {
                ctx: self.ctx,
                z3_tactic: t,
            }
        }
    }

    /// Return a tactic that applies itself to a given goal if the probe `p` evaluates to true.
    /// If `p` evaluates to false, then the new tactic behaves like the skip tactic.
    pub fn when(&self, p: &Probe<'ctx>) -> Tactic<'ctx> {
        unsafe {
            let t = Z3_tactic_when(self.ctx.z3_ctx, p.z3_probe, self.z3_tactic);
            Z3_tactic_inc_ref(self.ctx.z3_ctx, t);
            Tactic {
                ctx: self.ctx,
                z3_tactic: t,
            }
        }
    }

    /// Return a tactic that applies `t1` to a given goal if the probe `p` evaluates to true,
    /// and `t2` if `p` evaluates to false.
    pub fn cond(
        ctx: &'ctx Context,
        p: &Probe<'ctx>,
        t1: &Tactic<'ctx>,
        t2: &Tactic<'ctx>,
    ) -> Tactic<'ctx> {
        unsafe {
            let t = Z3_tactic_cond(ctx.z3_ctx, p.z3_probe, t1.z3_tactic, t2.z3_tactic);
            Z3_tactic_inc_ref(ctx.z3_ctx, t);
            Tactic { ctx, z3_tactic: t }
        }
    }

    /// Return a tactic that fails if the probe `p` evaluates to false.
    pub fn fail_if(ctx: &'ctx Context, p: &Probe<'ctx>) -> Tactic<'ctx> {
        unsafe {
            let t = Z3_tactic_fail_if(ctx.z3_ctx, p.z3_probe);
            Z3_tactic_inc_ref(ctx.z3_ctx, t);
            Tactic { ctx, z3_tactic: t }
        }
    }

    pub fn apply(&self, goal: &Goal<'ctx>, params: Option<&Params<'ctx>>) -> ApplyResult<'ctx> {
        ApplyResult {
            ctx: self.ctx,
            z3_apply_result: match params {
                None => unsafe {
                    let ar = Z3_tactic_apply(self.ctx.z3_ctx, self.z3_tactic, goal.z3_goal);
                    Z3_apply_result_inc_ref(self.ctx.z3_ctx, ar);
                    ar
                },
                Some(params) => unsafe {
                    let ar = Z3_tactic_apply_ex(
                        self.ctx.z3_ctx,
                        self.z3_tactic,
                        goal.z3_goal,
                        params.z3_params,
                    );
                    Z3_apply_result_inc_ref(self.ctx.z3_ctx, ar);
                    ar
                },
            },
        }
    }
}

impl<'ctx> fmt::Display for Tactic<'ctx> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let p = unsafe { Z3_tactic_get_help(self.ctx.z3_ctx, self.z3_tactic) };
        if p.is_null() {
            return Result::Err(fmt::Error);
        }
        match unsafe { CStr::from_ptr(p) }.to_str() {
            Ok(s) => write!(f, "{}", s),
            Err(_) => Result::Err(fmt::Error),
        }
    }
}

impl<'ctx> fmt::Debug for Tactic<'ctx> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        <Self as fmt::Display>::fmt(self, f)
    }
}

impl<'ctx> Drop for Tactic<'ctx> {
    fn drop(&mut self) {
        unsafe {
            Z3_tactic_dec_ref(self.ctx.z3_ctx, self.z3_tactic);
        }
    }
}
