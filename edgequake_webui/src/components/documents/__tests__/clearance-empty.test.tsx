/**
 * @vitest-environment jsdom
 */
import { DocumentTableStates } from "@/components/documents/document-table-states";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

describe("clearance empty library", () => {
  it("shows the profile sentence and no count", () => {
    render(
      <DocumentTableStates
        isLoading={false}
        isEmpty
        onUploadClick={() => undefined}
        clearanceEmpty
      />,
    );
    expect(screen.getByText("Nothing to show here")).toBeInTheDocument();
    expect(screen.getByText("An owner can give you an access profile.")).toBeInTheDocument();
    expect(screen.queryByText(/document/i)).not.toBeInTheDocument();
  });
});
