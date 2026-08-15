import {
  BREAKPOINT_NAMES,
  cloneLayout,
  type BreakpointName,
  type LayoutDocument,
  type WidgetManifest,
} from "@opensimdash/widget-sdk";
import { useEffect, useMemo, useRef, useState } from "preact/hooks";

import {
  LayoutApiError,
  LayoutConflictError,
  loadLayout,
  saveLayout,
} from "../api/layouts";
import { ConnectionScreen } from "../connection/screen";
import {
  gamePresentation,
  gamePresentations,
  isGamePluginId,
} from "../dashboard/game-profile";
import {
  dashboardThemeStyle,
  THEME_PRESETS,
  themePresetId,
} from "../dashboard/theme";
import { selectBreakpoint } from "../dashboard/breakpoint";
import { EditorCanvas } from "../editor/canvas";
import {
  addWidget,
  duplicateWidget,
  removeWidget,
  removeWidgetsByType,
  updateWidgetSettings,
} from "../editor/document";
import {
  clearLayoutDraft,
  loadLayoutDraft,
  saveLayoutDraft,
} from "../editor/draft";
import {
  commitHistory,
  createHistory,
  redoHistory,
  undoHistory,
} from "../editor/history";
import {
  createLayoutExport,
  importLayoutText,
  LayoutTransferError,
  MAX_LAYOUT_TRANSFER_BYTES,
} from "../editor/layout-transfer";
import { useTelemetryRuntime } from "../telemetry/use-runtime";
import {
  builtinWidgetManifest,
  builtinWidgetManifestsForPlugin,
} from "../widgets/catalog";
import "../styles/editor.css";

const BREAKPOINT_LABELS: Readonly<Record<BreakpointName, string>> = {
  phonePortrait: "手机竖屏",
  phoneLandscape: "手机横屏",
  tablet: "平板",
  desktop: "桌面预览",
};

const INITIAL_GAME_ID = "f1-24";

export function EditRoute() {
  const runtime = useTelemetryRuntime();
  const [selectedGameId, setSelectedGameId] = useState(INITIAL_GAME_ID);
  const presentations = useMemo(
    () => gamePresentations(runtime.plugins),
    [runtime.plugins],
  );
  const presentation = useMemo(
    () => gamePresentation(selectedGameId, runtime.plugins),
    [runtime.plugins, selectedGameId],
  );
  const [history, setHistory] = useState(() =>
    createHistory(cloneLayout(gamePresentation(INITIAL_GAME_ID, runtime.plugins).defaultLayout)),
  );
  const [savedDocument, setSavedDocument] = useState<LayoutDocument>();
  const [breakpoint, setBreakpoint] = useState<BreakpointName>(() =>
    selectBreakpoint(window.innerWidth, window.innerHeight),
  );
  const [selectedId, setSelectedId] = useState<string>();
  const [loadedLayoutId, setLoadedLayoutId] = useState<string>();
  const [saving, setSaving] = useState(false);
  const [notice, setNotice] = useState("正在读取 Host 布局…");
  const [conflict, setConflict] = useState<LayoutDocument>();
  const fileInput = useRef<HTMLInputElement>(null);
  const manuallySelectedGame = useRef(false);
  const demoMode = import.meta.env.DEV && new URLSearchParams(location.search).has("demo");
  const layout = history.present;
  const selected = layout.widgets.find((widget) => widget.instanceId === selectedId);
  const selectedManifest = selected ? builtinWidgetManifest(selected.componentType) : undefined;
  const availableManifests = builtinWidgetManifestsForPlugin(presentation.plugin);
  const dirty = savedDocument ? layoutSignature(layout) !== layoutSignature(savedDocument) : false;
  const loaded = loadedLayoutId === presentation.layoutId;

  useEffect(() => {
    if (presentations.some((candidate) => candidate.id === selectedGameId)) {
      return;
    }
    const fallback = presentations[0];
    if (fallback) {
      setSelectedGameId(fallback.id);
      setSelectedId(undefined);
      setConflict(undefined);
    }
  }, [presentations, selectedGameId]);

  useEffect(() => {
    if (
      manuallySelectedGame.current ||
      !isGamePluginId(runtime.gameId, runtime.plugins) ||
      runtime.gameId === selectedGameId
    ) {
      return;
    }
    if (dirty) {
      saveLayoutDraft(localStorage, layout);
    }
    setSelectedGameId(runtime.gameId);
    setSelectedId(undefined);
    setConflict(undefined);
  }, [dirty, layout, runtime.gameId, runtime.plugins, selectedGameId]);

  useEffect(() => {
    if (!runtime.hasConnected || loaded) {
      return;
    }
    let active = true;
    const fallback = cloneLayout(presentation.defaultLayout);
    setHistory(createHistory(fallback));
    setSavedDocument(fallback);
    setSelectedId(undefined);
    setConflict(undefined);
    setNotice(`正在读取 ${presentation.label} 的独立布局…`);
    if (demoMode) {
      setLoadedLayoutId(presentation.layoutId);
      setNotice("视觉演示模式：更改仅保存为本机浏览器草稿。");
      return;
    }
    void loadLayout(presentation.layoutId)
      .then((loadedLayout) => {
        if (!active) {
          return;
        }
        const server = loadedLayout.document;
        const draft = loadLayoutDraft(localStorage, server.id);
        if (draft && draft.baseRevision === server.revision) {
          setHistory(createHistory(draft.document));
          setNotice("已恢复这台设备上未保存的布局草稿。");
        } else if (draft) {
          setHistory(createHistory(draft.document));
          setConflict(server);
          setNotice("草稿基于旧版本，需要选择冲突处理方式。");
        } else {
          setHistory(createHistory(server));
          setNotice(loadedLayout.recovered ? "Host 已从最近备份恢复布局。" : "布局已同步。");
        }
        setSavedDocument(server);
        setLoadedLayoutId(presentation.layoutId);
      })
      .catch((error: unknown) => {
        if (!active) {
          return;
        }
        setLoadedLayoutId(presentation.layoutId);
        setSavedDocument(fallback);
        setNotice(
          error instanceof Error
            ? `无法读取布局，正在使用安全默认值：${error.message}`
            : "无法读取布局，正在使用安全默认值。",
        );
      });
    return () => {
      active = false;
    };
  }, [demoMode, loaded, presentation, runtime.hasConnected]);

  useEffect(() => {
    if (!loaded || !dirty) {
      return;
    }
    const timer = window.setTimeout(() => {
      if (!saveLayoutDraft(localStorage, layout)) {
        setNotice("浏览器没有足够空间保存布局草稿。");
      }
    }, 250);
    return () => window.clearTimeout(timer);
  }, [dirty, layout, loaded]);

  useEffect(() => {
    const handleKeyboard = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== "z") {
        return;
      }
      if (event.target instanceof HTMLInputElement || event.target instanceof HTMLSelectElement) {
        return;
      }
      event.preventDefault();
      setHistory((current) =>
        event.shiftKey ? redoHistory(current) : undoHistory(current),
      );
    };
    window.addEventListener("keydown", handleKeyboard);
    return () => window.removeEventListener("keydown", handleKeyboard);
  }, []);

  const commit = (next: LayoutDocument) => {
    setHistory((current) => commitHistory(current, next));
  };

  const selectGame = (nextGameId: string) => {
    if (nextGameId === selectedGameId) {
      return;
    }
    if (dirty && saveLayoutDraft(localStorage, layout)) {
      setNotice("已保存当前游戏草稿，正在切换面板。");
    }
    manuallySelectedGame.current = true;
    setSelectedGameId(nextGameId);
    setSelectedId(undefined);
    setConflict(undefined);
  };

  const toggleWidget = (manifest: WidgetManifest<object>) => {
    const instances = layout.widgets.filter(
      (widget) => widget.componentType === manifest.type,
    );
    if (instances.length > 0) {
      commit(removeWidgetsByType(layout, manifest.type));
      if (selected?.componentType === manifest.type) {
        setSelectedId(undefined);
      }
      setNotice(`已停用 ${manifest.displayName}。`);
      return;
    }
    const next = addWidget(layout, manifest);
    if (!next) {
      setNotice("所有预览断点都需要可用空间；请先停用或缩小一个组件。");
      return;
    }
    commit(next);
    setSelectedId(next.widgets.at(-1)?.instanceId);
    setNotice(`已启用 ${manifest.displayName}。`);
  };

  const duplicateSelected = () => {
    if (!selected || !selectedManifest) {
      return;
    }
    const next = duplicateWidget(layout, selected.instanceId, selectedManifest);
    if (!next) {
      setNotice("所有响应式断点都必须有可用空间，暂时无法复制。");
      return;
    }
    commit(next);
    setSelectedId(next.widgets.at(-1)?.instanceId);
  };

  const removeSelected = () => {
    if (!selected) {
      return;
    }
    commit(removeWidget(layout, selected.instanceId));
    setSelectedId(undefined);
  };

  const applySaved = (document: LayoutDocument, message: string) => {
    setHistory(createHistory(document));
    setSavedDocument(document);
    setConflict(undefined);
    clearLayoutDraft(localStorage, document.id);
    setNotice(message);
  };

  const persist = async (document: LayoutDocument, message: string) => {
    if (demoMode) {
      saveLayoutDraft(localStorage, document);
      setNotice("演示模式草稿已保存在当前浏览器。");
      return;
    }
    setSaving(true);
    try {
      const saved = await saveLayout(document);
      applySaved(saved.document, message);
    } catch (error: unknown) {
      if (error instanceof LayoutConflictError) {
        setConflict(error.current);
        setNotice("检测到另一台设备保存了更新版本。");
      } else {
        setNotice(
          error instanceof LayoutApiError || error instanceof Error
            ? `保存失败：${error.message}`
            : "保存失败，请稍后重试。",
        );
      }
    } finally {
      setSaving(false);
    }
  };

  const exportLayout = () => {
    const exported = createLayoutExport(layout);
    const url = URL.createObjectURL(
      new Blob([exported.content], { type: "application/json;charset=utf-8" }),
    );
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = exported.filename;
    anchor.hidden = true;
    document.body.append(anchor);
    anchor.click();
    anchor.remove();
    window.setTimeout(() => URL.revokeObjectURL(url), 0);
    setNotice(`已导出 ${exported.filename}。`);
  };

  const importLayout = async (file: File | undefined) => {
    if (!file) {
      return;
    }
    try {
      if (file.size > MAX_LAYOUT_TRANSFER_BYTES) {
        throw new LayoutTransferError("布局文件超过 256 KB 限制。");
      }
      const imported = importLayoutText(await file.text(), layout);
      commit(imported);
      setSelectedId(undefined);
      setNotice(`已导入 ${file.name}；确认预览后请保存到 Host。`);
    } catch (error: unknown) {
      setNotice(
        error instanceof Error ? `导入失败：${error.message}` : "导入失败：布局文件无效。",
      );
    } finally {
      if (fileInput.current) {
        fileInput.current.value = "";
      }
    }
  };

  if (!runtime.hasConnected) {
    return <ConnectionScreen view={runtime.connection} />;
  }

  return (
    <main class="editor-shell">
      <header class="editor-header">
        <div>
          <p class="eyebrow">OpenSimDash / Layout Lab</p>
          <strong>{layout.name}</strong>
        </div>
        <div class="editor-preview-controls">
          <label class="editor-breakpoint">
            <span>游戏面板</span>
            <select
              value={selectedGameId}
              onChange={(event) => selectGame(event.currentTarget.value)}
            >
              {presentations.map((profile) => (
                <option key={profile.id} value={profile.id}>
                  {profile.label}
                </option>
              ))}
            </select>
          </label>
          <label class="editor-breakpoint">
            <span>预览断点</span>
            <select
              value={breakpoint}
              onChange={(event) => setBreakpoint(event.currentTarget.value as BreakpointName)}
            >
              {BREAKPOINT_NAMES.map((name) => (
                <option key={name} value={name}>
                  {BREAKPOINT_LABELS[name]}
                </option>
              ))}
            </select>
          </label>
        </div>
        <nav class="editor-actions" aria-label="布局操作">
          <button
            type="button"
            disabled={history.past.length === 0}
            onClick={() => setHistory((current) => undoHistory(current))}
          >
            撤销
          </button>
          <button
            type="button"
            disabled={history.future.length === 0}
            onClick={() => setHistory((current) => redoHistory(current))}
          >
            重做
          </button>
          <input
            ref={fileInput}
            type="file"
            accept=".json,application/json"
            hidden
            aria-label="选择要导入的布局 JSON"
            onChange={(event) => void importLayout(event.currentTarget.files?.[0])}
          />
          <button type="button" onClick={() => fileInput.current?.click()}>
            导入
          </button>
          <button type="button" onClick={exportLayout}>
            导出
          </button>
          <button
            class="editor-save"
            type="button"
            disabled={!dirty || saving}
            onClick={() => void persist(layout, "布局已安全保存到 Host。")}
          >
            {saving ? "保存中…" : dirty ? "保存布局" : "已保存"}
          </button>
          <a href="/">返回驾驶</a>
        </nav>
      </header>

      <div class="editor-workspace">
        <aside class="editor-toolbox" aria-label="组件与样式">
          <section>
            <h2>组件</h2>
            <div class="editor-widget-catalog">
              {availableManifests.map((manifest) => {
                const instanceCount = layout.widgets.filter(
                  (widget) => widget.componentType === manifest.type,
                ).length;
                const enabled = instanceCount > 0;
                return (
                  <button
                    key={manifest.type}
                    type="button"
                    aria-label={`${enabled ? "停用" : "启用"} ${manifest.displayName}`}
                    aria-pressed={enabled}
                    onClick={() => toggleWidget(manifest)}
                  >
                    <span class="editor-widget-copy">
                      <strong>{manifest.displayName}</strong>
                      <small>
                        {enabled
                          ? instanceCount > 1
                            ? `已启用 · ${instanceCount} 个`
                            : "已启用"
                          : "未启用"}
                      </small>
                    </span>
                    <span class="editor-widget-switch" aria-hidden="true" />
                  </button>
                );
              })}
            </div>
          </section>

          <section>
            <h2>仪表主题</h2>
            <div class="editor-themes">
              {THEME_PRESETS.map((preset) => (
                <button
                  key={preset.id}
                  type="button"
                  aria-pressed={themePresetId(layout.theme) === preset.id}
                  onClick={() => commit({ ...layout, theme: { ...preset.theme } })}
                >
                  <span class={`theme-swatch theme-swatch-${preset.id}`} aria-hidden="true" />
                  {preset.name}
                </button>
              ))}
            </div>
          </section>

          <section class="editor-inspector">
            <h2>所选组件</h2>
            {selected ? (
              <>
                <p>
                  <strong>{selectedManifest?.displayName ?? selected.componentType}</strong>
                  <code>{selected.instanceId}</code>
                </p>
                {selected.componentType === "core.tachometer" ? (
                  <label>
                    <span>备用红线转速</span>
                    <input
                      type="number"
                      min="1000"
                      max="30000"
                      step="100"
                      value={Number(selected.settings.fallbackRpmMax ?? 12_000)}
                      onChange={(event) =>
                        commit(
                          updateWidgetSettings(layout, selected.instanceId, {
                            fallbackRpmMax: Math.min(
                              30_000,
                              Math.max(1_000, event.currentTarget.valueAsNumber || 12_000),
                            ),
                          }),
                        )
                      }
                    />
                  </label>
                ) : null}
                <div class="editor-inspector-actions">
                  <button type="button" onClick={duplicateSelected}>
                    复制
                  </button>
                  <button class="editor-danger" type="button" onClick={removeSelected}>
                    移除
                  </button>
                </div>
              </>
            ) : (
              <p class="editor-hint">点击组件顶部手柄进行选择和拖动。</p>
            )}
          </section>
        </aside>

        <section class="editor-stage" aria-label={`${BREAKPOINT_LABELS[breakpoint]}布局预览`}>
          <div class={`editor-canvas-frame editor-canvas-${breakpoint}`}>
            <div
              class="drive-dashboard editor-preview"
              data-stale="false"
              data-theme={themePresetId(layout.theme)}
              data-game={presentation.id}
              data-game-family={presentation.family}
              style={dashboardThemeStyle(layout.theme)}
            >
              <div class="dashboard-frame" aria-hidden="true" />
              <EditorCanvas
                layout={layout}
                breakpoint={breakpoint}
                loop={runtime.loop}
                connection={runtime.connection}
                statusMode={presentation.statusMode}
                selectedId={selectedId}
                onSelect={setSelectedId}
                onCommit={commit}
              />
              <footer class="drive-footer">
                <span>{presentation.label} / {BREAKPOINT_LABELS[breakpoint]} / EDIT MODE</span>
                <span>REV {layout.revision}</span>
              </footer>
            </div>
          </div>
        </section>
      </div>

      <footer class="editor-status" aria-live="polite">
        <span data-phase={runtime.connection.phase}>{runtime.connection.phase}</span>
        <p>{notice}</p>
        <strong>{dirty ? "有未保存更改" : "与 Host 同步"}</strong>
      </footer>

      {conflict ? (
        <div class="editor-conflict-backdrop">
          <section class="editor-conflict" role="dialog" aria-modal="true" aria-labelledby="conflict-title">
            <p class="eyebrow">Revision conflict</p>
            <h2 id="conflict-title">另一台设备已修改此布局</h2>
            <p>
              Host 当前为 revision {conflict.revision}。请选择加载服务器版本，或明确用当前草稿覆盖它。
            </p>
            <div>
              <button
                type="button"
                onClick={() => applySaved(conflict, "已加载 Host 上的最新布局。")}
              >
                加载服务器版本
              </button>
              <button
                class="editor-danger"
                type="button"
                disabled={saving}
                onClick={() =>
                  void persist(
                    { ...layout, revision: conflict.revision },
                    "已按你的选择覆盖 Host 布局。",
                  )
                }
              >
                覆盖为我的草稿
              </button>
            </div>
          </section>
        </div>
      ) : null}
    </main>
  );
}

function layoutSignature(document: LayoutDocument): string {
  return JSON.stringify(document);
}
