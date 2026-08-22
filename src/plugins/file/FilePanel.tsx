import { useCallback, useEffect, useMemo, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  AlertTriangle,
  ArrowRight,
  Ban,
  CheckCircle2,
  CircleDashed,
  ExternalLink,
  FilePlus2,
  FileText,
  FolderOpen,
  LoaderCircle,
  Play,
  RotateCcw,
  ShieldCheck,
  Trash2,
  X,
} from "lucide-react";
import type {
  FileConversionCandidate,
  FileConversionDirection,
  FileConversionJobSnapshot,
  FileConversionQualityProfile,
} from "./contracts";
import {
  fileConversionJobStateKey,
  fileConversionProviderGuidance,
  fileConversionQueueActions,
  planFileConversionIntake,
  reconcileFileConversionCandidates,
  summarizeFileConversionJob,
} from "./fileModel";
import {
  fileErrorTranslationKey,
  type TranslationKey,
} from "./i18n";
import {
  useFileConversion,
  type FileConversionAction,
  type FileConversionController,
} from "./useFileConversion";

interface FilePanelProps {
  t: (key: TranslationKey) => string;
}

const DIRECTIONS: FileConversionDirection[] = ["pdfToDocx", "docxToPdf"];

export function FilePanel({ t }: FilePanelProps) {
  const controller = useFileConversion();
  const [isDragActive, setIsDragActive] = useState(false);
  const queueActions = useMemo(
    () => fileConversionQueueActions(controller.jobs, controller.capabilities),
    [controller.jobs, controller.capabilities],
  );

  const addCandidates = useCallback(
    async (candidates: FileConversionCandidate[]) => {
      const intake = planFileConversionIntake(candidates);
      if (intake.enqueueItems.length === 0) {
        controller.setCandidates(candidates);
        return;
      }
      const result = await controller.enqueue(intake.enqueueItems);
      if (result) {
        controller.setCandidates(
          reconcileFileConversionCandidates(
            intake.rejectedCandidates,
            result.rejectedCandidates,
          ),
        );
      }
    },
    [controller.enqueue, controller.setCandidates],
  );

  const inspectPaths = useCallback(
    async (paths: string[]) => {
      if (paths.length === 0) return;
      const candidates = await controller.inspect(paths);
      if (candidates) await addCandidates(candidates);
    },
    [addCandidates, controller.inspect],
  );

  const chooseFiles = useCallback(async () => {
    const candidates = await controller.choose();
    if (candidates) await addCandidates(candidates);
  }, [addCandidates, controller.choose]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    getCurrentWindow()
      .onDragDropEvent(({ payload }) => {
        if (disposed) return;
        if (payload.type === "enter" || payload.type === "over") {
          setIsDragActive(true);
        } else if (payload.type === "leave") {
          setIsDragActive(false);
        } else {
          setIsDragActive(false);
          void inspectPaths(payload.paths);
        }
      })
      .then((disposeListener) => {
        if (disposed) disposeListener();
        else unlisten = disposeListener;
      })
      .catch(() => {
        // Browser-only previews do not expose the native Tauri drag bridge.
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [inspectPaths]);

  return (
    <section className="plugin-panel file-panel accent-file" aria-labelledby="file-panel-title">
      <header className="file-panel-header">
        <div>
          <span className="file-kicker">LOCAL / SESSION</span>
          <h2 id="file-panel-title">{t("file.title")}</h2>
        </div>
        <div className="file-header-actions">
          <span className="status-pill">
            {controller.jobs.length > 0
              ? `${controller.jobs.length} ${t("file.queueCount")}`
              : t("file.formats")}
          </span>
          {controller.jobs.length > 0 ? (
            <button
              type="button"
              className="file-icon-action"
              title={t("action.addMore")}
              aria-label={t("action.addMore")}
              disabled={controller.busy.choose === true}
              onClick={() => void chooseFiles()}
            >
              <FilePlus2 aria-hidden="true" />
            </button>
          ) : null}
        </div>
      </header>

      <ProviderStrip controller={controller} t={t} />

      {controller.isLoading ? (
        <QueueSkeleton t={t} />
      ) : (
        <>
          <button
            type="button"
            className={`file-drop-zone ${isDragActive ? "drag-active" : ""}`}
            aria-label={t("file.dropTitle")}
            disabled={controller.busy.choose === true || controller.busy.inspect === true}
            onClick={() => void chooseFiles()}
          >
            <span className="file-drop-icon"><FilePlus2 aria-hidden="true" /></span>
            <span>
              <strong>{isDragActive ? t("file.dropActive") : t("file.dropTitle")}</strong>
              <small>{t("file.dropHint")}</small>
            </span>
            <span className="file-drop-action">{t("action.choose")}</span>
          </button>

          {controller.error &&
          (!controller.actionError || controller.actionError.owner === true) ? (
            <div className="file-inline-alert" role="alert">
              <AlertTriangle aria-hidden="true" />
              <span>{t(fileErrorTranslationKey(controller.error.code))}</span>
            </div>
          ) : null}

          <div className="file-queue-region" data-testid="file-queue-region">
            {controller.candidates.length > 0 ? (
              <CandidateFeedback
                candidates={controller.candidates}
                t={t}
                onDismiss={(sourcePath) =>
                  controller.setCandidates((current) =>
                    current.filter((candidate) => candidate.sourcePath !== sourcePath),
                  )
                }
              />
            ) : null}

            {controller.jobs.length > 0 ? (
              <div className="file-job-list" aria-label={t("file.queueTitle")} aria-live="polite">
                {controller.jobs.map((job) => (
                  <FileJobRow key={job.id} job={job} controller={controller} t={t} />
                ))}
              </div>
            ) : (
              <div className="file-empty-state" role="status">
                <div className="file-empty-rail" aria-hidden="true">
                  <FormatNode label="PDF" />
                  <span><ArrowRight /></span>
                  <FormatNode label="DOCX" />
                </div>
                <strong>{t("file.noQueueTitle")}</strong>
                <span>{t("file.noQueueHint")}</span>
              </div>
            )}
          </div>
        </>
      )}

      <footer className="file-panel-footer">
        <div className="file-privacy-note">
          <ShieldCheck aria-hidden="true" />
          <span>{t("file.localNote")}</span>
        </div>
        <div className="file-batch-actions">
          <button
            type="button"
            className="file-text-action"
            disabled={!queueActions.canClearCompleted || controller.busy.clear === true}
            onClick={() => void controller.clearCompleted()}
          >
            <Trash2 aria-hidden="true" />
            {t("action.clearCompleted")}
          </button>
          <button
            type="button"
            className="file-primary-action"
            disabled={!queueActions.canStart || controller.busy.start === true}
            onClick={() => void controller.start()}
          >
            {controller.busy.start === true ? (
              <LoaderCircle aria-hidden="true" />
            ) : (
              <Play aria-hidden="true" />
            )}
            {controller.busy.start === true
              ? t("action.converting")
              : t("action.convertAll")}
          </button>
        </div>
      </footer>
    </section>
  );
}

function ProviderStrip({ controller, t }: { controller: FileConversionController; t: FilePanelProps["t"] }) {
  return (
    <div className="file-provider-strip" aria-label={t("file.provider")}>
      {DIRECTIONS.map((direction) => {
        const guidance = fileConversionProviderGuidance(controller.capabilities, direction);
        return (
          <div className={guidance.available ? "available" : "unavailable"} key={direction}>
            {guidance.available ? <CheckCircle2 aria-hidden="true" /> : <CircleDashed aria-hidden="true" />}
            <span>
              <strong>{direction === "pdfToDocx" ? "PDF → DOCX" : "DOCX → PDF"}</strong>
              <small>
                {guidance.available
                  ? `${guidance.providerName ?? t("provider.available")} · ${
                      guidance.providerOrigin === "builtIn"
                        ? t("provider.offlineReady")
                        : t("provider.compatibility")
                    }`
                  : t("provider.repair")}
              </small>
            </span>
          </div>
        );
      })}
    </div>
  );
}

function QueueSkeleton({ t }: { t: FilePanelProps["t"] }) {
  return (
    <div className="file-loading-state" role="status" aria-live="polite">
      <div>
        <LoaderCircle aria-hidden="true" />
        <span><strong>{t("file.loading")}</strong><small>{t("file.loadingHint")}</small></span>
      </div>
      {[0, 1].map((item) => (
        <div className="file-skeleton-row" key={item} aria-hidden="true">
          <i /><span /><b />
        </div>
      ))}
    </div>
  );
}

function CandidateFeedback({
  candidates,
  t,
  onDismiss,
}: {
  candidates: FileConversionCandidate[];
  t: FilePanelProps["t"];
  onDismiss: (sourcePath: string) => void;
}) {
  return (
    <section className="file-candidate-feedback" aria-labelledby="file-candidate-title">
      <h3 id="file-candidate-title">{t("file.reviewTitle")}</h3>
      {candidates.map((candidate) => {
        const rejected = candidate.validation.status === "rejected";
        return (
          <div className={rejected ? "rejected" : "valid"} key={candidate.sourcePath} role={rejected ? "alert" : "status"}>
            {rejected ? <AlertTriangle aria-hidden="true" /> : <CheckCircle2 aria-hidden="true" />}
            <span>
              <strong title={candidate.sourceName}>{candidate.sourceName}</strong>
              <small>
                {candidate.validation.status === "rejected"
                  ? t(fileErrorTranslationKey(candidate.validation.error.code))
                  : `${t("file.target")}: ${candidate.validation.proposedOutputName}`}
              </small>
            </span>
            <button type="button" title={t("action.dismiss")} aria-label={`${t("action.dismiss")}: ${candidate.sourceName}`} onClick={() => onDismiss(candidate.sourcePath)}>
              <X aria-hidden="true" />
            </button>
          </div>
        );
      })}
    </section>
  );
}

function FileJobRow({ job, controller, t }: { job: FileConversionJobSnapshot; controller: FileConversionController; t: FilePanelProps["t"] }) {
  const summary = summarizeFileConversionJob(job, controller.capabilities, "main");
  const stateKey = fileConversionJobStateKey(job, controller.capabilities);
  const rowError =
    job.state.status === "failed" || job.state.status === "cancelled"
      ? job.state.error
      : controller.actionError?.owner === job.id
        ? controller.actionError.error
        : undefined;
  const isBusy = (action: FileConversionAction) => controller.busy[action] === job.id;
  const stateIcon = stateIconFor(job, stateKey);

  return (
    <article className={`file-job-row state-${stateKey.slice("state.".length)}`} data-state={stateKey}>
      <div className="file-job-transfer">
        <div className="file-job-source">
          <span className="file-format-mark"><FileText aria-hidden="true" /></span>
          <span>
            <strong title={job.sourceName}>{job.sourceName}</strong>
            <small>{formatFileSize(job.sizeBytes)}</small>
          </span>
        </div>
        <div className="file-job-rail" aria-label={t(summary.directionKey)}>
          <span />
          <ArrowRight aria-hidden="true" />
        </div>
        <div className="file-job-target">
          <strong title={summary.outputName ?? summary.targetName}>{summary.outputName ?? summary.targetName}</strong>
          <small>{t(summary.directionKey)}</small>
        </div>
      </div>

      <div className="file-job-state">
        <span className="file-state-icon" aria-hidden="true">{stateIcon}</span>
        <span>
          <strong>{t(summary.stateKey)}</strong>
          <small>
            {summary.stageKey
              ? t(summary.stageKey)
              : summary.providerName ??
                (stateKey === "state.engineUnavailable"
                  ? t(job.direction === "pdfToDocx" ? "provider.pdfToDocxUnavailable" : "provider.docxToPdfUnavailable")
                  : t("file.provider"))}
          </small>
        </span>
        {summary.percent !== undefined ? (
          <span
            className="file-progress"
            role="progressbar"
            aria-label={t("file.progress")}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={summary.percent}
          >
            <i style={{ width: `${summary.percent}%` }} />
            <b>{summary.percent}%</b>
          </span>
        ) : job.state.status === "running" ? (
          <span className="file-progress indeterminate" role="progressbar" aria-label={`${t("file.progress")}: ${t("file.indeterminate")}`}>
            <i />
          </span>
        ) : null}
      </div>

      {rowError ? (
        <div className="file-row-error" role="alert">
          <AlertTriangle aria-hidden="true" />
          <span>{t(fileErrorTranslationKey(rowError.code))}</span>
        </div>
      ) : null}

      <div className="file-job-details">
        <span><b>{t("file.provider")}</b>{summary.providerName ?? "—"}</span>
        <span><b>{t("file.output")}</b>{summary.outputName ?? summary.targetName}</span>
        <span>
          {job.state.status === "completed"
            ? t(qualityTranslationKey(job.state.result.qualityProfile))
            : t("file.fidelityNote")}
        </span>
      </div>

      <div className="file-job-actions">
        {summary.actions.canOpen ? (
          <button type="button" disabled={isBusy("open")} onClick={() => void controller.open(job.id)}>
            <ExternalLink aria-hidden="true" />{t("action.open")}
          </button>
        ) : null}
        {summary.actions.canReveal ? (
          <button type="button" disabled={isBusy("reveal")} onClick={() => void controller.reveal(job.id)}>
            <FolderOpen aria-hidden="true" />{t("action.reveal")}
          </button>
        ) : null}
        {summary.actions.canCancel ? (
          <button type="button" disabled={isBusy("cancel")} onClick={() => void controller.cancel(job.id)}>
            <Ban aria-hidden="true" />{t("action.cancel")}
          </button>
        ) : null}
        {summary.actions.canRetry ? (
          <button type="button" disabled={isBusy("retry")} onClick={() => void controller.retry(job.id)}>
            <RotateCcw aria-hidden="true" />{t("action.retry")}
          </button>
        ) : null}
        {summary.actions.canRemove ? (
          <button type="button" disabled={isBusy("remove")} onClick={() => void controller.remove(job.id)}>
            <Trash2 aria-hidden="true" />{t("action.remove")}
          </button>
        ) : null}
      </div>
    </article>
  );
}

function qualityTranslationKey(profile: FileConversionQualityProfile): TranslationKey {
  const keys: Record<FileConversionQualityProfile, TranslationKey> = {
    editableReconstruction: "quality.editableReconstruction",
    layoutPreserving: "quality.layoutPreserving",
    webRenderedPdf: "quality.webRenderedPdf",
    compatibilityProvider: "quality.compatibilityProvider",
  };
  return keys[profile];
}

function stateIconFor(job: FileConversionJobSnapshot, stateKey: string) {
  if (stateKey === "state.engineUnavailable") return <CircleDashed />;
  switch (job.state.status) {
    case "completed": return <CheckCircle2 />;
    case "failed": return <AlertTriangle />;
    case "cancelled": return <Ban />;
    case "running":
    case "preparing": return <LoaderCircle />;
    default: return <CircleDashed />;
  }
}

function FormatNode({ label }: { label: string }) {
  return <span className="file-format-node"><FileText aria-hidden="true" /><b>{label}</b></span>;
}

function formatFileSize(bytes: number | undefined) {
  if (bytes === undefined) return "—";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
