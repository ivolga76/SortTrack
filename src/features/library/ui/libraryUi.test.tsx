import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { AppContent, AppShell } from "../../../app/AppShell";
import { LibraryFilters } from "./LibraryFilters";
import { LibraryHeading } from "./LibraryHeading";
import { LibraryPagination } from "./LibraryPagination";
import { LibrarySelectionActions } from "./LibrarySelectionActions";

const noOp = () => undefined;

describe("library feature UI boundaries", () => {
  it("keeps the application and content shell classes stable", () => {
    const html = renderToStaticMarkup(<AppShell><span>sidebar</span><AppContent>library</AppContent><span>player</span></AppShell>);
    expect(html).toBe('<div class="app-shell"><span>sidebar</span><main class="content">library</main><span>player</span></div>');
  });

  it("renders the incoming heading and real review fallback", () => {
    const html = renderToStaticMarkup(<LibraryHeading incomingOnly total={148} reviewCount={36} genreBusy={false} onReviewGenres={noOp}/>);
    expect(html).toContain("Новые поступления");
    expect(html).toContain("Входящие");
    expect(html).toContain("36 требуют проверки");
  });

  it("renders advanced filters only when explicitly opened", () => {
    const props = {
      genre: null,
      activeAdvancedFilterCount: 0,
      minBpm: "",
      maxBpm: "",
      minRating: "",
      camelot: "",
      format: "" as const,
      fileStatus: "all" as const,
      selectionActions: <span>actions</span>,
      onGenreChange: noOp,
      onAdvancedFiltersOpenChange: noOp,
      onMinBpmChange: noOp,
      onMaxBpmChange: noOp,
      onMinRatingChange: noOp,
      onCamelotChange: noOp,
      onFormatChange: noOp,
      onFileStatusChange: noOp,
      onReset: noOp,
    };
    expect(renderToStaticMarkup(<LibraryFilters {...props} advancedFiltersOpen={false}/>)).not.toContain("Расширенные фильтры библиотеки");
    expect(renderToStaticMarkup(<LibraryFilters {...props} advancedFiltersOpen/>)).toContain("Расширенные фильтры библиотеки");
  });

  it("disables pagination and bulk rating actions at their boundaries", () => {
    const pagination = renderToStaticMarkup(<LibraryPagination incomingOnly offset={0} total={100} pageSize={100} onIncomingOnlyChange={noOp} onOffsetChange={noOp}/>);
    expect((pagination.match(/disabled/g) ?? [])).toHaveLength(2);

    const actions = renderToStaticMarkup(<LibrarySelectionActions
      checkedCount={0}
      visibleCount={100}
      total={100000}
      sortBy="added"
      descending
      ratingTarget="-1"
      ratingBusy={false}
      datasetBusy={false}
      genreBusy={false}
      metadataLoading={false}
      playlistCount={2}
      onSortByChange={noOp}
      onDescendingChange={noOp}
      onRatingTargetChange={noOp}
      onApplyRating={noOp}
      onOpenDataset={noOp}
      onOpenGenres={noOp}
      onOpenAnalysis={noOp}
      onOpenMetadata={noOp}
      onOpenPlaylists={noOp}
    />);
    expect(actions).toContain("100 из 100000");
    expect(actions).toContain("Плейлисты · 2");
    expect(actions).toContain("<button disabled=\"\">Применить</button>");
  });
});
