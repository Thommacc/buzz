# Workforce Avatar Rollout Design

**Date:** 2026-08-21  
**Status:** Approved for implementation planning  
**Scope:** The 35 portable Buzz employee identities

## Goal

Create one durable, recognizable profile image for each of the 35 employee
identities and publish the matching name and image profile to every active Buzz
community where that identity is available. The same employee must retain the
same face and public key across communities.

## Visual Direction

The avatars use a human, professional photographic style with enough playfulness
to remain pleasant and easy to distinguish in a long conversation list.

- Depict 35 distinct adult women, matching the existing female employee names.
- Use broad natural variation in age, skin tone, facial features, hair, clothing,
  and personality.
- Frame every portrait head-and-shoulders, front-facing or subtly turned, with
  the face centered for a small circular crop.
- Use soft studio or window-style lighting and a restrained neutral background.
- Add one subtle playful detail per portrait, such as colorful glasses, earrings,
  a scarf, a fabric pattern, a warm expression, or a gentle background accent.
- Keep clothing professional but varied rather than applying one uniform.
- Avoid text, logos, watermarks, caricatures, costumes, role stereotypes, and
  distracting props.
- Produce square 1024 by 1024 PNG source assets.

Department accents may influence clothing or the background, but must not make
the image company-specific. An employee image remains reusable in Thommacc Labs,
TotalTools, Van der Hilst, Damen, DSRY, and future communities.

## Identity Mapping

The rollout covers exactly these existing workforce identities:

1. Billie Billing
2. Cassie Case Study
3. Collette Collections
4. Connie Content
5. Coppy Copywriting
6. Della Deadline
7. Dottie Documentation
8. Drea Drafting
9. Elena Expansion
10. Enya Enquiry
11. Faye Follow-up
12. Hanna Handover
13. Ines Inbox
14. Kira Knowledge
15. Mette Meeting
16. Nora Nurture
17. Onna Onboarding
18. Orla Outreach
19. Pia Pipeline
20. Pippa Prospecting
21. Posy Positioning
22. Prena Prep
23. Prodi Production
24. Quinn Quality
25. Reina Research
26. Renee Renewals
27. Retta Retention
28. Reva Referral
29. Rhea Reactivation
30. Romy Reporting
31. Skye Scoping
32. Sophie Scope
33. Tessa Templating
34. Tia Testimonial
35. Vera Visibility

Every asset receives a stable filename derived from `identityId`. A manifest
maps the identity ID, display name, public key, role, department, source asset,
uploaded media URL, and per-relay publication result. No private key or auth tag
may appear in the manifest or logs.

## Generation and Review

Start with three canary portraits chosen from different departments. Review them
for style consistency, face diversity, circular-crop safety, and playful detail.
After the canaries pass, generate the remaining 32 using the same locked visual
specification while varying individual appearance deliberately.

Each output must pass these checks before publication:

- valid PNG;
- exactly 1024 by 1024 pixels;
- unique file hash;
- no visible text, logo, or watermark;
- one clearly visible adult subject;
- face and key features remain legible in a small circular crop;
- filename and manifest identity agree.

## Storage and Upload

Final source assets and the non-secret manifest live in a durable local workforce
avatar directory, outside transient image-generation storage. Existing assets are
never overwritten silently; replacements receive a new version and require a
manifest update.

Upload each approved PNG through the target Buzz relay's supported media path.
Record only a confirmed durable URL. A failed upload stops publication for that
identity on that relay and is recorded without substituting another image.

## Profile Publication

For each active relay and employee identity:

1. Read the current public profile.
2. Preserve unrelated profile fields.
3. Publish the exact workforce display name and confirmed avatar URL with the
   employee's own signing key.
4. Require an accepted write response.
5. Read the profile back from the same relay and compare public key, display name,
   and avatar URL with the manifest.

The current active rollout target is Thommacc Labs, TotalTools, and Van der Hilst.
Damen and DSRY remain excluded from live publication until their relays and
community rollout state are active and compatible. Their future publication uses
the same manifest and images, not regenerated faces.

## Safety, Backup, and Rollback

Before mutation, save a timestamped backup of the workforce store, managed-agent
store, and any current public profile data that exists. Never print or persist
private signing keys in reports. Keep WSL, Hermes, and running agent processes
online; avatar publication does not require a restart.

Rollback republishes the backed-up profile fields for affected identities and
restores the local stores if they changed. Uploaded media may remain present but
must no longer be referenced after rollback.

## Verification and Completion

The rollout is complete only when:

- all 35 source PNG files and manifest rows exist;
- every identity has a unique image and the intended display name;
- each active relay returns the expected name and avatar for all 35 public keys;
- representative existing messages from Thomas, Sophie, and Steven resolve names
  correctly without regressing their separate Hermes identities;
- desktop displays names and avatars after refresh;
- mobile receives the same relay-backed profile data after refresh;
- no WSL/Hermes service or unrelated Buzz profile was restarted or modified;
- the applicable agent and system journals contain the rollout, backup, impact,
  rollback, and verification evidence.

