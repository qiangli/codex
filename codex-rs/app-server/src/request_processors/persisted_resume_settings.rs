use codex_protocol::config_types::ApprovalsReviewer;
use codex_protocol::models::ActivePermissionProfile;
use codex_protocol::models::BUILT_IN_PERMISSION_PROFILE_DANGER_FULL_ACCESS;
use codex_protocol::models::PermissionProfile;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::EventMsg;
use codex_rollout::RolloutItem;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct PersistedResumeSettings {
    pub(super) approval_policy: AskForApproval,
    pub(super) approvals_reviewer: Option<ApprovalsReviewer>,
    pub(super) active_permission_profile: Option<ActivePermissionProfile>,
}

pub(super) fn latest_persisted_resume_settings(
    history: &[RolloutItem],
) -> Option<PersistedResumeSettings> {
    history
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, item)| match item {
            RolloutItem::TurnContext(turn_context) => Some(PersistedResumeSettings {
                approval_policy: turn_context.approval_policy,
                approvals_reviewer: turn_context.approvals_reviewer.or_else(|| {
                    history[..index].iter().rev().find_map(|item| match item {
                        RolloutItem::TurnContext(turn_context) => turn_context.approvals_reviewer,
                        RolloutItem::EventMsg(EventMsg::ThreadSettingsApplied(event)) => {
                            Some(event.thread_settings.approvals_reviewer)
                        }
                        _ => None,
                    })
                }),
                active_permission_profile: turn_context.active_permission_profile.clone().or_else(
                    || {
                        // A turn-level sandbox override can have no profile identity. Recover the
                        // thread's own full-access selection from its latest settings checkpoint,
                        // rather than treating it as an absent selection on cold resume.
                        history[..index]
                            .iter()
                            .rev()
                            .find_map(|item| match item {
                                RolloutItem::EventMsg(EventMsg::ThreadSettingsApplied(event)) => {
                                    Some(full_access_profile(
                                        &event.thread_settings.permission_profile,
                                    ))
                                }
                                _ => None,
                            })
                            .flatten()
                    },
                ),
            }),
            RolloutItem::EventMsg(EventMsg::ThreadSettingsApplied(event)) => {
                Some(PersistedResumeSettings {
                    approval_policy: event.thread_settings.approval_policy,
                    approvals_reviewer: Some(event.thread_settings.approvals_reviewer),
                    active_permission_profile: event
                        .thread_settings
                        .active_permission_profile
                        .clone()
                        .or_else(|| full_access_profile(&event.thread_settings.permission_profile)),
                })
            }
            _ => None,
        })
}

fn full_access_profile(profile: &PermissionProfile) -> Option<ActivePermissionProfile> {
    matches!(profile, PermissionProfile::Disabled)
        .then(|| ActivePermissionProfile::new(BUILT_IN_PERMISSION_PROFILE_DANGER_FULL_ACCESS))
}

#[cfg(test)]
#[path = "persisted_resume_settings_tests.rs"]
mod tests;
