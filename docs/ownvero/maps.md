# Maps and accessible lists

**Evidence state: development-source guidance; no approved owner-workspace installer or completed native acceptance.**

Maps explain explicit evidence relationships. They do not infer business behavior
from visual proximity, a file diff, or a dependency-looking line.

## Change journey

Follow Intent, Preserve, Brief, Verify, and Decide. Each step points to its actual
record or missing prerequisite. An animation never advances a request or makes a
check pass. Open the blocking issue to find the required next action.

## Capability and evidence map

Read capabilities, confirmed rules, scenarios, and observed results as separate
objects. Links name their meaning: requires, verifies, and observed_by. An unmapped
effect remains coverage unknown. Selecting a capability shows only its explicit
path; missing connections are not filled in by inference.

## Compare before and after

Keep build labels visible and compare the same scenario and rule revisions. When
expectations differ, treat the results as incomparable until you review that diff.
Removing scenarios is not an improvement in coverage or a passing percentage.

## Optional 3D workbench

The three layers are owner intent, test scenarios, and observed results. Separating
layers makes the relationships easier to see; geometric distance carries no
confidence or causality score. Select a card to read the same inspector used by
the 2D view. Reset, fit selected, zoom, and layer controls change the view only.

The public synthetic demo loads 3D only when requested. It has no idle animation
loop. Close the workbench to release its resources. A WebGL error returns you to
the complete 2D cards and connection list.

## Keyboard and fallback

Use ordinary buttons, expandable cards, and the connection list without dragging
or interpreting color. Labels carry the status; green alone never means owner
acceptance. The demo has no automatic motion and works with reduced motion.
Documentation and the 2D reading view remain available without JavaScript.
