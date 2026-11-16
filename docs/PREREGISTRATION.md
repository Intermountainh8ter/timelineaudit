# timelineaudit pre-registration

Registered 2026-11-16, before any claim was entered in the table. This file
is never edited. Later hypotheses, and every claim, are appended to
`preregistration_additions.md` as dated sections before they are scored.

## The question

Several widely read documents say that artificial general intelligence, and
with it a catastrophe or a transformation of everything, is a few years away.
The best known is "AI 2027" (Kokotajlo, Alexander, Larsen, Lifland and Dean,
AI Futures Project, April 2025), whose scenario reaches its crisis in 2027.
Whether the ending is right cannot be checked before it arrives, and this
study does not try to. It asks a narrower question that can be answered
today: **of the dated, checkable claims these documents make on the way to
their ending, how many have come true on time, how late are the rest, and
were the stated probabilities any better than a coin?**

The same question is asked of the documents that say the opposite. A source
that predicts slow progress makes dated claims too, and they are scored by
the same rules.

## Scope of any conclusion

- No result here licenses the sentence "AGI will arrive by" any date, "AGI
  will not arrive by" any date, "the scenario is right" or "the scenario is
  wrong". Results are stated as the track record of dated claims: how many,
  from which source, due when, resolved how.
- A good record on early claims does not make a later claim true. Early
  claims are nearer and easier; the audit shows this effect and every result
  reports short-lead and long-lead claims apart.
- A poor record does not make a later claim false. A source can be wrong
  about dates and right about direction, and the slip measure exists to show
  that case.
- Nothing here measures risk. A claim such as "this would end in
  catastrophe" has no date on which it can be checked without the catastrophe
  and is recorded as not checkable, never scored.
- Sources are scored on what they wrote, at the date they wrote it. Where the
  authors later revised a date, the revision is entered as a new claim with
  its own date, and both are kept.

## Sources

The first source is the AI 2027 scenario and its supplements
(https://ai-2027.com). Others are added one at a time, each in a dated
section of `preregistration_additions.md` that names the document, its
publication date and the extraction rule, before any of its claims is looked
up. Candidates the research scan checks: Aschenbrenner, "Situational
Awareness" (June 2024); the task-length trend in Kwa et al., "Measuring AI
Ability to Complete Long Software Tasks" (arXiv:2503.14499); Narayanan and
Kapoor, "AI as Normal Technology" (2025), as a source of claims in the other
direction; forecasting-platform questions with a published community
probability; and dated public statements by AI-lab leaders. Each scan
re-checks these and anything that supersedes them, including the authors'
own later revisions.

## Extraction rule

Cherry-picking is the main way this study could mislead, so claims are not
chosen; they are extracted.

1. A section of a source is named in `preregistration_additions.md` before
   it is read for claims.
2. Every sentence in that section that asserts something will have happened
   by a date is entered, including the ones that are awkward to resolve.
3. For each claim the entry records: the exact wording, the URL, the date
   the source was published, the due date and how it was read from the text
   (a claim dated "mid-2026" is due 2026-06-30, "late 2026" is due
   2026-12-31, "early 2026" is due 2026-03-31, a bare year is due on its
   31 December), the probability if the source gave one, and a resolution
   rule: what public evidence would count as the event having happened.
4. The resolution rule is written before the outcome is looked up. An entry
   whose rule is changed after the lookup is marked `unclear` and stays in
   the table.
5. A claim that cannot be given a resolution rule is counted in the
   section's `not checkable` total and left out of the table. That total is
   reported beside every result for the source.
6. The outcome is the date the event was first publicly documented, with the
   URL of the evidence, or `-` if no such evidence is found, or `unclear`.

## Measures

All are computed by `timelineaudit score` from one claim table and a scoring
date given on the command line. The unit of analysis is one claim; intervals
are percentile bootstraps over claims with 2000 resamples and the seed
recorded in the results file. Every measure is reported for all claims, for
each source and for each class (`capability`, `deployment`, `economy`,
`governance`, `safety`, `other`).

### M1. Hit rate (`/groups/*/hit_rate`)

Of the claims that are resolved (the event happened, or its date has
passed), the share whose event happened on or before the stated date. Open
and unclear claims are counted beside it and not scored.

- Reported with `hit_rate_short_lead` (due within 365 days of being made)
  and `hit_rate_long_lead`.
- No hypothesis is attached. The hit rate depends on how hard the claims
  were as much as on how good the source was, so it is never compared
  between sources as a measure of skill.

### M2. Timing slip (`/groups/*/median_slip_months`)

For each resolved claim, the observed date minus the stated date, in months
of 30.4375 days. Negative means earlier than stated. An overdue event that
has not happened contributes the time overdue so far as a lower bound, and
`slip_is_lower_bound` is true whenever such a bound could change the median.

- H2: for the AI 2027 scenario's `capability` claims the median slip is
  above zero, with the lower end of the interval above zero: stated dates
  for capabilities run early.
- Null value: 0. Smallest effect of interest: 3 months.

### M3. Brier skill (`/groups/*/brier_skill_vs_coin`)

For resolved claims that carried a probability, the mean squared difference
between the probability and the outcome, and one minus that over 0.25, the
score of a forecaster that says 0.5 to everything.

- H3: across all sources' probabilistic claims, skill is above zero with the
  lower end of the interval above zero.
- Null value: 0. Smallest effect of interest: 0.05.
- A group with fewer than 20 such claims reports the number and no
  conclusion.

## Degenerate-forecaster audit

Before any source is scored, `timelineaudit baselines` scores five simulated
forecasters on one simulated world in which every event has its own true
chance of happening by its date: one that states the true chance, one that
says yes to everything, one that says no to everything, a coin, and one that
states the true chance but puts every date 90 days too early. The audit
passes when only the first shows Brier skill with an interval above zero,
when the first four share one hit rate (the hit rate cannot see
probabilities, which is why M1 carries no hypothesis), and when the last
shows a median slip exactly 90 days above the first. A measure that a
know-nothing forecaster can pass is withdrawn, not reinterpreted. The audit
is in `tests/audit.rs` and its output is `results/000-baselines.json`.

## Analysis rules

- Confirmatory analyses are H2 and H3 as stated. Anything else is labelled
  exploratory in the results file and in any text.
- The scoring date is an argument and is recorded. The system clock is never
  used to resolve a claim.
- A claim whose event had already happened when the claim was published is
  rejected by the parser: it is not a forecast.
- Claims are never removed. A wrong entry is corrected by a dated note in
  `preregistration_additions.md` naming the old and new values.
- Results that favour the alarming reading and results that favour the
  calming one are reported with the same prominence, with the claim counts
  beside them.
- Text about a result names the sources and the scoring date and does not
  describe a source's authors, only their dated claims.

## Claim table

Tab-separated, one row per claim, in `claims/claims.tsv`:
`id source class made_on due_on prob outcome url`.

| column | content |
|---|---|
| `id` | unique label, also used in `preregistration_additions.md` |
| `source` | short label for the document |
| `class` | `capability`, `deployment`, `economy`, `governance`, `safety` or `other` |
| `made_on` | publication date of the claim, `YYYY-MM-DD` |
| `due_on` | date by which the event was said to happen |
| `prob` | stated probability from 0 to 1, or `-` |
| `outcome` | date the event was first documented, `-` if not yet, or `unclear` |
| `url` | where the claim was published |
