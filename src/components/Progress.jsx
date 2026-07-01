import {motion} from "framer-motion";
import {Download, Cog, ShieldCheck, Loader2, Lock, HardDrive} from "lucide-react";
import {formatBytes, formatRate, formatEta} from "../lib/format.js";

const PHASE = {
    auth: {label: "Waiting for permission…", Icon: Lock},
    prepare: {label: "Preparing…", Icon: Loader2},
    download: {label: "Downloading & writing", Icon: Download},
    write: {label: "Writing", Icon: HardDrive},
    finalize: {label: "Finalizing…", Icon: Cog},
    verify: {label: "Verifying", Icon: ShieldCheck},
};

const Progress = ({progress, onCancel}) => {
    const {phase, done, total, rate} = progress;
    const pct = total > 0 ? Math.min(100, Math.round((done / total) * 100)) : 0;
    const remaining = total > done ? total - done : 0;
    const info = PHASE[phase] || {label: "Working…", Icon: Loader2};
    const Icon = info.Icon;

    return (
        <div className="progress">
            <div className="progress__phase">
                <Icon size={18} className={phase === "prepare" ? "spin" : ""}/>
                {info.label}
            </div>
            <div className="progress__sub">
                {phase === "auth"
                    ? "Approve the administrator prompt to continue."
                    : phase === "verify"
                        ? "Reading the device back to make sure every byte matches."
                        : phase === "prepare"
                            ? "Getting things ready…"
                            : "Do not unplug the drive or close this window."}
            </div>

            <div className="progress__track">
                <motion.div
                    className="progress__bar"
                    animate={{width: `${total > 0 ? pct : 100}%`}}
                    transition={{ease: "linear", duration: 0.2}}
                />
            </div>

            <div className="progress__pct">{total > 0 ? `${pct}%` : "…"}</div>

            <div className="progress__stats">
        <span>
          {formatBytes(done)}
            {total > 0 ? ` / ${formatBytes(total)}` : ""}
        </span>
                <span>{formatRate(rate)}</span>
                <span>ETA {formatEta(remaining, rate)}</span>
            </div>

            <button
                className="pbtn pbtn--ghost"
                style={{marginTop: 26}}
                onClick={onCancel}
            >
                Cancel
            </button>
        </div>
    );
}

export default Progress;
