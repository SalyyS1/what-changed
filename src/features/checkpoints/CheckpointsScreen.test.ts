import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import { empty, failed, loading, ready } from "../../lib/types/async-state";
import type { CheckpointDto } from "../../lib/tauri/bindings";
import CheckpointsScreen from "./CheckpointsScreen.svelte";
import ComparisonView from "./ComparisonView.svelte";
import { checkpointRow } from "./checkpoint-row";

const partial: CheckpointDto = {
  id: 7,
  name: "Before graphics update",
  kind: "manual",
  reason: "Updating the GPU driver",
  created_at: Date.now() - 60_000,
  item_count: 2874,
  duration_ms: 4921,
  partial: true,
  missing_collectors: ["drivers"],
};

describe("CheckpointsScreen states", () => {
  it("uses the required empty-state copy", () => {
    render(CheckpointsScreen, { data: empty<CheckpointDto[]>() });
    expect(screen.getByText("No checkpoints yet.")).toBeTruthy();
    expect(
      screen.getByText(
        "Create one while things work, so you can compare later.",
      ),
    ).toBeTruthy();
  });

  it("covers loading and error states without exposing raw internals", async () => {
    const view = render(CheckpointsScreen, {
      data: loading<CheckpointDto[]>(),
    });
    expect(screen.getAllByTestId("skeleton").length).toBeGreaterThan(0);
    await view.rerender({
      data: failed<CheckpointDto[]>("Checkpoint storage is unavailable."),
    });
    expect(screen.getByText("Checkpoint storage is unavailable.")).toBeTruthy();
  });

  it("labels partial rows and names the missing collector", () => {
    render(CheckpointsScreen, { data: ready([partial]) });
    expect(screen.getByText("Partial")).toBeTruthy();
    expect(
      screen.getByText("Partial — Drivers could not be captured"),
    ).toBeTruthy();
    expect(screen.getByRole("button", { name: "Compare to now" })).toBeTruthy();
  });
});

describe("ComparisonView", () => {
  it("groups all three diff kinds by category and links the event window", () => {
    render(ComparisonView, {
      checkpoint: checkpointRow(partial),
      diff: {
        added: [
          {
            category: "application",
            entity_display_name: "Toolbox",
            field: null,
            before: null,
            after: null,
          },
        ],
        removed: [
          {
            category: "service",
            entity_display_name: "Old Agent",
            field: null,
            before: null,
            after: null,
          },
        ],
        changed: [
          {
            category: "driver",
            entity_display_name: "Display adapter",
            field: "version",
            before: "1.0",
            after: "2.0",
          },
        ],
        grouped_by_category: [
          {
            category: "application",
            added: [
              {
                category: "application",
                entity_display_name: "Toolbox",
                field: null,
                before: null,
                after: null,
              },
            ],
            removed: [],
            changed: [],
          },
          {
            category: "service",
            added: [],
            removed: [
              {
                category: "service",
                entity_display_name: "Old Agent",
                field: null,
                before: null,
                after: null,
              },
            ],
            changed: [],
          },
          {
            category: "driver",
            added: [],
            removed: [],
            changed: [
              {
                category: "driver",
                entity_display_name: "Display adapter",
                field: "version",
                before: "1.0",
                after: "2.0",
              },
            ],
          },
        ],
        event_ids_in_window: [11, 12],
        window_from: 1_800_000_000_000,
        window_to: 1_800_000_060_000,
        left_missing_collectors: ["drivers"],
        right_missing_collectors: ["network"],
        skipped_collectors: ["drivers", "eventlog"],
      },
      onback: () => {},
    });
    expect(screen.getByRole("heading", { name: "Added" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Removed" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Changed" })).toBeTruthy();
    expect(
      screen.getByText("Drivers not compared — missing from this checkpoint"),
    ).toBeTruthy();
    expect(
      screen.getByText("Network not compared — missing from the current state"),
    ).toBeTruthy();
    expect(
      screen.getByText(
        "Windows event log not compared — comparison is unavailable for this collector",
      ),
    ).toBeTruthy();
    expect(
      screen
        .getByRole("link", { name: /2 changes recorded in this window/ })
        .getAttribute("href"),
    ).toBe("#/timeline?range=custom&from=1800000000000&to=1800000060000");
    expect(screen.getByText("Show technical details")).toBeTruthy();
  });

  it("labels an unsupported collector as a partial comparison, not a partial checkpoint", () => {
    render(ComparisonView, {
      checkpoint: checkpointRow({
        ...partial,
        partial: false,
        missing_collectors: [],
      }),
      diff: {
        added: [],
        removed: [],
        changed: [],
        grouped_by_category: [],
        event_ids_in_window: [],
        window_from: 1,
        window_to: 2,
        left_missing_collectors: [],
        right_missing_collectors: [],
        skipped_collectors: ["eventlog"],
      },
      onback: () => {},
    });
    expect(screen.getByText("This comparison is partial.")).toBeTruthy();
    expect(
      screen.getByText(
        "Windows event log not compared — comparison is unavailable for this collector",
      ),
    ).toBeTruthy();
  });
});
