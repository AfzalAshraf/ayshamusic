# A little comfort — optional feature proposal

Status: the user explicitly requested a Quotes tab in the bottom navigation. A first version is now implemented in src/index.html, preserving the approved player. The original proposal below remains context; implementation details and limits are recorded here and in MOBILE_UI.md.

## Purpose

A separate, gentle space Aysha can choose to open when she wants a comforting message. Keep the approved music layout and exact dedication unchanged. Offer warmth and autonomy, not a claim to cure anxiety, guarantee safety in every situation or change how someone feels about a partner.

## Proposed experience

- A separate optional entry/card, not an intrusive popup or replacement for music controls.
- Short original messages under Rest, Kindness, Gentle encouragement and Affection.
- Controls: Next message, Save/Like, Show more like this, category selection and Hide this space.
- No forced reading, notifications, timed auto-rotation, guilt, relationship score or streaks. Do not interrupt music or move a message while she is reading.
- No claims that she owes affection, must stay with a partner or can only find comfort through the app. Do not label her mood or mental health from clicks.
- No fabricated shared memories or quotes falsely attributed to real people. Obtain supplied personal wording if a message needs a specific memory or promise.

## Personalization recommendation

Ask Aysha in the interface whether she wants saved likes to tailor messages. If she opts in, rank categories using only her explicit on-device likes/Show more actions. A liked category may appear more often, with variety and an option to choose something different; it is not a diagnosis or a relationship measurement.

Use browser-local storage only for the initial version. Explain what is stored, keep it separate from music state and other people's devices, and provide Reset preferences/Delete saved messages. If storage is unavailable, show unpersonalized messages normally. Turning personalization off must stop preference-based ranking.

Do not silently measure dwell time, eye attention or time spent looking. Those measures do not establish which quote helps someone and could expose sensitive emotional preferences. They are not included in this proposal. No analytics, remote AI service, server mood profile or sharing her preferences with another person is proposed.

## Original sample messages

- "A small pause is allowed. You don’t have to finish everything today."
- "You deserve kindness on the days that feel untidy, too."
- "There is room for your feelings, and you can take this moment at your own pace."
- "I’m glad you’re here. There’s no need to be anything other than yourself."
- "You are free to say what you need, and to take your time."
- "A gentle moment together can be enough; it doesn’t have to be perfect."

These are supportive messages, not treatment or predictions about anxiety or a relationship. If she wants help beyond the app, it should not discourage trusted human or professional support.

## Approval and acceptance gates

Confirm the opt-in, explicit-likes-only approach before the UI department implements it. Then verify:

1. The feature can be skipped/hidden, is accessible on Android and does not affect playback/Back history.
2. No personalization data is collected before opt-in or sent off-device.
3. Likes actually alter the selected category distribution; Reset and turning off personalization work.
4. Corrupt/missing/unavailable storage does not break the music player.
5. Copy remains supportive and non-coercive; no cure/safety/affection guarantee or mental-health inference.
6. The approved baseline, exact dedication and resource constraints remain intact.


## Implemented first version

- Fifth navigation item: Quotes, between Library and Bluetooth (desktop sidebar uses the same item).
- Separate “A little comfort” page with 24 original messages, categories Love/Rest/Kindness/Courage, For you, Saved, Next and save/unsave.
- No timer, view/dwell/mood tracking, autoplay, message notifications or remote quote API. Browsing is stateless with respect to persistent storage until an explicit save/setting action.
- Personalization is off by default, controlled by a labeled checkbox. When on, the For you mix weights explicit saved categories and More like this actions, retaining variety and avoiding immediate repeats. Turning it off stops all preference weighting. It does not infer feelings or promise a cure/attachment/safety outcome.
- A bounded dedicated browser-local record, aysha.comfort.v1, stores only version, saved IDs, bounded category counts and opt-in. Saved/setting choices can survive reload; corrupt/unavailable storage falls back to in-memory behavior. Reset removes this record without touching the music selection. Same browser/profile users can see its local choices; no cross-device sync or account storage is provided.
- Quotes can be skipped by choosing another tab; there is no hide-navigation setting in this first version because the user explicitly requested the visible bottom tab. Details collapse the preference controls. No other navigation is removed.
- The music element/source is shared and untouched, Back/Forward supports the tab, and direct #quotes reload works without search/audio APIs. Sandbox UI preview can show Quotes while its live audio backend remains unavailable.

Automated evidence: 33 frontend tests and seven Chromium viewports, including five navigation targets at >=44px, local categories/saves/reset, opt-in/reload behavior, weighted-selection response, malformed/denied storage and no audio restart on tab use. Actual Android/iPhone accessibility/hardware/music acceptance remains a separate gate.
