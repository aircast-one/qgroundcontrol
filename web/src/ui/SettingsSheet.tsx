import { type ReactNode, type RefObject, useContext, useEffect, useLayoutEffect, useMemo, useRef, useState, useSyncExternalStore, type CSSProperties } from 'react';
import { useBackHandler, useKeyedState, useLaunchedEffect, useUpdatedRef } from '@/kotlin/Compose';
import { type MutableState, mutableStateOf, useValue } from '@/kotlin/State';
import { HorizontalDivider, IconButton, ScrollableTabRow, SnackbarHost, Tab, Text, TextButton, useSnackbarHostState } from '@/m3';
import { type DrawableName } from '@/res/drawables';
import { Drawable } from '@/res/Icon';
import { AircastSpace, aircast, md, withAlpha } from '@/theme/Colors';
import { LocalAppNavigation } from '@/ui/AppNavigation';
import { osdShadow } from '@/ui/Osd';
import { LocalPageHeading, type PageHeading, SearchPill } from '@/ui/SetupRow';
import { DETAIL_PANE_MAX_WIDTH, useDesktopLayout } from '@/ui/DesktopLayout';
import { SetupScreen } from '@/ui/SetupScreen';
import { type ChangeNotice, LocalChangeNotice } from '@/ui/ValueRow';
import { AIRCRAFT_SETUP, SettingsGroup, SettingsGroupEntries, SettingsScreen, SettingsSearch, pageLook, pageTitle, useRememberSettingsPages } from '@/ui/SettingsScreen';

export const sheetDrilled = (openPage: string | null, setupOpen: boolean): boolean => openPage !== null || setupOpen;

export const openingGroup = (requested: string | null): SettingsGroup => (requested === null ? SettingsGroup.General : pageLook(requested).group);

const LANDSCAPE = '(orientation: landscape)';

const useLandscape = (): boolean =>
  useSyncExternalStore(
    (listener) => {
      const query = window.matchMedia(LANDSCAPE);
      query.addEventListener('change', listener);
      return () => query.removeEventListener('change', listener);
    },
    () => window.matchMedia(LANDSCAPE).matches,
    () => false,
  );

const REDUCED_MOTION = '(prefers-reduced-motion: reduce)';
const SETTLE_MS = 120;

type PagerState = {
  readonly element: RefObject<HTMLDivElement | null>;
  readonly targetPage: number;
  readonly currentPage: () => number;
  readonly animatingTo: () => number | null;
  readonly animateScrollToPage: (page: number) => void;
  readonly scrollToPage: (page: number) => void;
  readonly onScroll: (onSettled: (page: number) => void) => void;
};

const usePagerState = (initialPage: number): PagerState => {
  const element = useRef<HTMLDivElement | null>(null);
  const animating = useRef<number | null>(null);
  const settling = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [targetPage, setTargetPage] = useState(initialPage);
  return useMemo(() => {
    const nearest = (): number => {
      const pane = element.current;
      return pane === null || pane.clientWidth === 0 ? 0 : Math.round(pane.scrollLeft / pane.clientWidth);
    };
    const scroll = (page: number, behavior: ScrollBehavior) => element.current?.scrollTo({ left: page * element.current.clientWidth, behavior });
    return {
      element,
      targetPage,
      currentPage: nearest,
      animatingTo: () => animating.current,
      animateScrollToPage: (page) => {
        animating.current = page;
        setTargetPage(page);
        scroll(page, window.matchMedia(REDUCED_MOTION).matches ? 'instant' : 'smooth');
      },
      scrollToPage: (page) => {
        animating.current = null;
        setTargetPage(page);
        scroll(page, 'instant');
      },
      onScroll: (onSettled) => {
        setTargetPage(animating.current ?? nearest());
        if (settling.current !== null) clearTimeout(settling.current);
        settling.current = setTimeout(() => {
          animating.current = null;
          setTargetPage(nearest());
          onSettled(nearest());
        }, SETTLE_MS);
      },
    };
  }, [targetPage]);
};

export const SettingsSheet = ({ requested, onClose }: { requested: string | null; onClose: () => void }) => {
  const navigation = useContext(LocalAppNavigation);
  const aircraftRequested = useValue(navigation.aircraftRequested);
  const settingsPage = useValue(navigation.settingsPage);
  const [group, setGroup] = useKeyedState([requested], () => (navigation.aircraftRequested.value ? SettingsGroup.General : openingGroup(requested)));
  const pager = usePagerState(group.ordinal);
  const [setupOpen, setSetupOpen] = useState(() => navigation.aircraftRequested.value);
  const [setupFromTab, setSetupFromTab] = useState(false);
  const [enteredForSetup, setEnteredForSetup] = useState(() => navigation.aircraftRequested.value);
  useEffect(() => {
    if (!setupOpen) setEnteredForSetup(false);
  }, [setupOpen]);
  const [page, setPage] = useKeyedState<string | null>([requested], () => requested);
  const [query, setQuery] = useState<string | null>(null);
  const [returnQuery, setReturnQuery] = useState<string | null>(null);
  const backToSearch = () => {
    setQuery(returnQuery);
    setReturnQuery(null);
    setPage(null);
  };
  const closeSetup = () => {
    if (enteredForSetup) {
      onClose();
      return;
    }
    setSetupOpen(false);
    if (returnQuery !== null) backToSearch();
  };
  useLaunchedEffect([aircraftRequested], () => {
    if (!aircraftRequested) return;
    const asked = navigation.setupPage.value;
    setSetupFromTab((it) => it || (asked !== null && asked !== ''));
    setSetupOpen(true);
    setQuery(null);
    navigation.aircraftRequested.value = false;
  });
  useLaunchedEffect([settingsPage], () => {
    if (settingsPage === null) return;
    setGroup(pageLook(settingsPage).group);
    setPage(settingsPage);
    setSetupOpen(false);
    setQuery(null);
    navigation.settingsPage.value = null;
  });
  useEffect(() => {
    navigation.settingsShowing.value = setupOpen ? null : group;
    return () => {
      navigation.settingsShowing.value = null;
    };
  }, [group, setupOpen]);
  useBackHandler(true, onClose);
  useBackHandler(query !== null, () => setQuery(null));
  useBackHandler(setupOpen && query === null, closeSetup);
  const initialPage = page !== null && pageLook(page).group === group ? page : null;
  const [openPage, setOpenPage] = useKeyedState<string | null>([group, page, query !== null, setupOpen], () => (initialPage !== null && !pageLook(initialPage).inline ? initialPage : null));
  const closePage = () => {
    if (returnQuery !== null && openPage === initialPage) backToSearch();
    else setOpenPage(null);
  };
  const [pageHeading] = useState(() => mutableStateOf<PageHeading | null>(null));
  const heading = useValue(pageHeading);
  const pageBack = () => (heading !== null ? heading.back() : closePage());
  const setupBack = () => (heading !== null && !setupFromTab ? heading.back() : closeSetup());
  const openSetup = (component: string | null) => {
    navigation.setupPage.value = component;
    setSetupFromTab(component !== null);
    setSetupOpen(true);
    setReturnQuery(query !== null && query.trim() !== '' ? query : null);
    setQuery(null);
  };
  const snackbars = useSnackbarHostState();
  const latestSnackbars = useUpdatedRef(snackbars);
  const notice = useMemo<ChangeNotice>(
    () => ({
      show: (message, undo) =>
        void (async () => {
          latestSnackbars.current.current?.settle('Dismissed');
          const result = await latestSnackbars.current.showSnackbar(message, undo !== null ? 'Undo' : null, 'Long');
          if (result !== 'ActionPerformed' || undo === null) return;
          const refusal = await undo();
          if (refusal !== null) await latestSnackbars.current.showSnackbar(`Undo failed: ${refusal}`);
        })(),
    }),
    [],
  );
  const landscape = useLandscape();
  const groups = SettingsGroupEntries;
  const pickTab = (entry: SettingsGroup) => {
    setGroup(entry);
    setPage(null);
    setSetupOpen(false);
    setReturnQuery(null);
    setQuery(null);
  };
  const drilled = sheetDrilled(openPage, setupOpen);
  const tabsShown = query === null && !drilled;
  const drilledTitle =
    openPage !== null && query === null && !setupOpen ? (heading?.title ?? pageTitle(openPage)) : query === null && setupOpen ? (heading?.title ?? AIRCRAFT_SETUP) : null;
  const showTab = (entry: SettingsGroup) => {
    pickTab(entry);
    pager.animateScrollToPage(entry.ordinal);
  };
  const contentKey = `${group.name}|${page ?? ''}|${query !== null}|${setupOpen}`;
  const desktop = useDesktopLayout();
  const content =
    query === null && !setupOpen ? (
      <SettingsTabsPager pager={pager} groups={groups} group={group} swipeable={openPage === null && !desktop} onSettled={pickTab} className="min-h-0 flex-1">
        {(shown, everyPage) => {
          const current = shown === group;
          return (
            <SettingsScreen
              key={current && page !== null ? `page:${page}` : 'none'}
              group={shown}
              everyPage={everyPage}
              open={current ? openPage : null}
              onOpen={setOpenPage}
              onClose={closePage}
              heading={pageHeading}
              className="h-full w-full"
              onOpenSetup={openSetup}
            />
          );
        }}
      </SettingsTabsPager>
    ) : query !== null ? (
      <SettingsSearch
        key={contentKey}
        query={query}
        className="min-h-0 flex-1"
        onOpenSetup={openSetup}
        onOpen={(title) => {
          setGroup(pageLook(title).group);
          setPage(title);
          setReturnQuery(pageLook(title).inline ? null : query);
          setQuery(null);
          setSetupOpen(false);
        }}
      />
    ) : (
      <SetupBranch key={contentKey} heading={pageHeading} enabled={setupFromTab && query === null} onBack={setupBack} />
    );
  const snackbarHost = (
    <div className={`pointer-events-none absolute inset-x-0 bottom-0 flex pb-[max(16px,env(safe-area-inset-bottom))] ${desktop ? 'justify-start px-menu' : 'justify-center'}`}>
      <SnackbarHost hostState={snackbars} />
    </div>
  );

  if (desktop)
    return (
      <div className="fixed inset-0 bg-surface text-on-surface" style={{ zIndex: SETTINGS_SHEET_LAYER }}>
        <LocalChangeNotice.Provider value={notice}>
          <div className="relative flex h-full w-full">
            <nav aria-label="Settings" className="flex w-[300px] shrink-0 flex-col gap-section overflow-y-auto bg-surface-container-low px-menu py-page">
              <Text typography="titleLarge" className="px-menu pt-2">
                Settings
              </Text>
              <SearchPill value={query ?? ''} onValueChange={(value) => setQuery(value === '' ? null : value)} placeholder="Search settings" />
              <SideTabs groups={groups} selected={query === null && !setupOpen ? group : null} onPick={pickTab} />
            </nav>
            <div className="relative flex min-w-0 flex-1 flex-col">
              <div className={`flex w-full items-center gap-2 pt-page pb-2 ${setupOpen ? 'px-page pr-16' : 'px-list'}`} style={setupOpen ? undefined : { maxWidth: DETAIL_PANE_MAX_WIDTH }}>
                {drilledTitle !== null ? (
                  <IconButton onClick={setupOpen ? setupBack : pageBack} label="Back" className="-ml-2">
                    <Drawable name="ic_arrow_back" />
                  </IconButton>
                ) : null}
                <Text typography="headlineSmall" maxLines={1} ellipsis className="min-w-0 flex-1">
                  {drilledTitle ?? (query !== null ? 'Search results' : group.title)}
                </Text>
              </div>
              <IconButton onClick={onClose} label="Close settings" className="absolute top-page right-page">
                <Drawable name="ic_close" />
              </IconButton>
              {content}
            </div>
            {snackbarHost}
          </div>
        </LocalChangeNotice.Provider>
      </div>
    );

  return (
    <div
      className="fixed inset-0 bg-(--settings-backdrop) text-on-surface"
      style={{ zIndex: SETTINGS_SHEET_LAYER, '--settings-backdrop': withAlpha(md('surface'), landscape ? SETTINGS_PANEL_ALPHA : 1) } as CSSProperties}
    >
      <LocalChangeNotice.Provider value={notice}>
        <div className="relative h-full w-full">
          <div className="flex h-full w-full flex-col pt-[env(safe-area-inset-top)] pr-[env(safe-area-inset-right)] pb-[env(safe-area-inset-bottom)] pl-[env(safe-area-inset-left)]">
            <div className="@container">
              <div
                className={`flex w-full items-center ${tabsShown ? 'pb-1 @min-[600px]:pb-0' : 'pb-1'}`}
                style={{ gap: AircastSpace.s2, paddingLeft: drilledTitle !== null ? AircastSpace.s1 : AircastSpace.s3, paddingRight: AircastSpace.s3, paddingTop: AircastSpace.s1 }}
              >
                {query !== null ? (
                  <>
                    <FocusedOnShow className="min-w-0 flex-1">
                      <SearchPill value={query} onValueChange={setQuery} placeholder="Search settings" />
                    </FocusedOnShow>
                    <TextButton onClick={() => setQuery(null)}>Cancel</TextButton>
                  </>
                ) : (
                  <>
                    {drilledTitle !== null ? (
                      <>
                        <IconButton onClick={setupOpen ? setupBack : pageBack} label="Back">
                          <Drawable name="ic_arrow_back" />
                        </IconButton>
                        <Text typography="titleLarge" maxLines={1} ellipsis className="min-w-0 flex-1">
                          {drilledTitle}
                        </Text>
                      </>
                    ) : (
                      <>
                        <Text typography="titleLarge" className="flex-1 pl-2 @min-[600px]:hidden">
                          Settings
                        </Text>
                        <div className="hidden min-w-0 flex-1 @min-[600px]:block">
                          <div className="mx-auto max-w-[720px]">
                            <SheetTabs groups={groups} selected={pager.targetPage} edgePadding={0} onPick={showTab} />
                          </div>
                        </div>
                      </>
                    )}
                    <IconButton onClick={() => setQuery('')} label="Search settings">
                      <Drawable name="ic_search" />
                    </IconButton>
                  </>
                )}
                <IconButton onClick={onClose} label="Close settings">
                  <Drawable name="ic_close" />
                </IconButton>
              </div>
              {tabsShown ? (
                <div className="w-full @min-[600px]:hidden">
                  <SheetTabs groups={groups} selected={pager.targetPage} edgePadding={AircastSpace.s2} onPick={showTab} />
                </div>
              ) : null}
              {tabsShown ? <HorizontalDivider /> : null}
            </div>
            {content}
          </div>
          {snackbarHost}
        </div>
      </LocalChangeNotice.Provider>
    </div>
  );
};

const SetupBranch = ({ heading, enabled, onBack }: { heading: MutableState<PageHeading | null>; enabled: boolean; onBack: () => void }) => {
  useBackHandler(enabled, onBack);
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <LocalPageHeading.Provider value={heading}>
        <SetupScreen className="min-h-0 flex-1" />
      </LocalPageHeading.Provider>
    </div>
  );
};

const FocusedOnShow = ({ className, children }: { className: string; children: ReactNode }) => {
  const holder = useRef<HTMLDivElement>(null);
  useEffect(() => {
    holder.current?.querySelector<HTMLInputElement>('input')?.focus();
  }, []);
  return (
    <div ref={holder} className={className}>
      {children}
    </div>
  );
};

const SettingsTabsPager = ({
  pager,
  groups,
  group,
  swipeable,
  onSettled,
  className,
  children,
}: {
  pager: PagerState;
  groups: readonly SettingsGroup[];
  group: SettingsGroup;
  swipeable: boolean;
  onSettled: (group: SettingsGroup) => void;
  className: string;
  children: (group: SettingsGroup, everyPage: ReturnType<typeof useRememberSettingsPages>) => ReactNode;
}) => {
  const everyPage = useRememberSettingsPages();
  const shownGroup = useUpdatedRef(group);
  const settled = useUpdatedRef(onSettled);
  useLayoutEffect(() => {
    if (pager.currentPage() !== group.ordinal && pager.animatingTo() !== group.ordinal) pager.scrollToPage(group.ordinal);
  }, [group]);
  const land = (page: number) => {
    const landed = groups[page];
    if (landed !== undefined && landed !== shownGroup.current) settled.current(landed);
  };
  const near = (index: number) => Math.abs(index - pager.targetPage) <= 1 || index === group.ordinal;
  return (
    <div
      ref={pager.element}
      onScroll={() => pager.onScroll(land)}
      className={`flex w-full snap-x snap-mandatory [scrollbar-width:none] ${swipeable ? 'overflow-x-auto' : 'overflow-x-hidden'} overflow-y-hidden ${className}`}
    >
      {groups.map((entry, index) => (
        <div key={entry.name} inert={entry !== group} aria-hidden={entry !== group || undefined} className="h-full w-full shrink-0 snap-start snap-always overflow-hidden">
          {near(index) ? children(entry, everyPage) : null}
        </div>
      ))}
    </div>
  );
};

const SheetTabs = ({ groups, selected, edgePadding, onPick }: { groups: readonly SettingsGroup[]; selected: number; edgePadding: number; onPick: (group: SettingsGroup) => void }) => (
  <ScrollableTabRow selectedTabIndex={selected} className="border-b-0!">
    <span className="shrink-0" style={{ width: edgePadding }} />
    {groups.map((entry) => (
      <Tab key={entry.name} selected={entry.ordinal === selected} onClick={() => onPick(entry)} text={<span className="whitespace-nowrap">{entry.title}</span>} />
    ))}
    <span className="shrink-0" style={{ width: edgePadding }} />
  </ScrollableTabRow>
);

const GROUP_ICONS: Record<SettingsGroup['name'], DrawableName> = {
  General: 'ic_tune',
  Safety: 'ic_shield',
  Control: 'ic_gamepad',
  Camera: 'ic_videocam',
  Transmission: 'ic_wifi',
  Device: 'ic_developer_board',
};

const SideTabs = ({ groups, selected, onPick }: { groups: readonly SettingsGroup[]; selected: SettingsGroup | null; onPick: (group: SettingsGroup) => void }) => (
  <div role="tablist" aria-orientation="vertical" className="flex flex-col gap-1">
    {groups.map((entry) => (
      <button
        key={entry.name}
        type="button"
        role="tab"
        aria-selected={entry === selected}
        onClick={() => onPick(entry)}
        className={`flex h-14 shrink-0 items-center gap-3 rounded-full px-list text-label-large ${entry === selected ? 'bg-secondary-container text-on-secondary-container' : 'text-on-surface-variant hover:bg-[color-mix(in_srgb,currentColor_8%,transparent)]'}`}
      >
        <Drawable name={GROUP_ICONS[entry.name]} />
        {entry.title}
      </button>
    ))}
  </div>
);

const SETTINGS_SHEET_LAYER = 10;
const SETTINGS_PANEL_ALPHA = 0.9;

export const FlySettingsButton = () => {
  const navigation = useContext(LocalAppNavigation);
  return (
    <IconButton onClick={() => (navigation.settingsOpen.value = true)} label="Settings">
      <Text typography="titleLarge" color={aircast('outdoorForeground')} className={osdShadow()}>
        ⋯
      </Text>
    </IconButton>
  );
};
