use super::{SignalFlags, MAX_SIG};

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
/// We align it to 16 bytes so that it doesn't cross virtual pages.
///
/// [Mention]: It should be noted that our current implementation is relatively simple
/// and does not support signal nesting for the time being,
/// that is, to execute another signal processing routine
/// during the execution of one signal processing routine.
/// 
/// (same as user/lib/SignalAction)
pub struct SignalAction {
  /// Represents the entry address of the signal processing routine.
  pub handler: usize,
  /// Indicates the signal `mask` during execution of the signal processing routine.
  pub mask: SignalFlags,
}

impl Default for SignalAction {
  fn default() -> Self {
    Self {
      handler: 0,
      mask: SignalFlags::empty(),
    }
  }
}

#[derive(Clone)]
pub struct SignalActions {
  pub table: [SignalAction; MAX_SIG + 1],
}

impl Default for SignalActions {
  fn default() -> Self {
    Self {
      table: [SignalAction::default(); MAX_SIG + 1],
    }
  }
}
