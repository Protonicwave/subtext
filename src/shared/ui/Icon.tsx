import type { ReactNode, SVGProps } from 'react';

/**
 * The icons, drawn rather than imported.
 *
 * A dozen line icons at one weight is less code than an icon library and none
 * of its bundle, and it keeps the stroke consistent with the rest of the
 * design. They inherit colour and take their size from the caller.
 *
 * Every one is decorative: the control around it carries the name. That is why
 * they are hidden from assistive technology here rather than being labelled
 * one at a time.
 */

type IconProps = Omit<SVGProps<SVGSVGElement>, 'children'> & {
  /** Width and height in pixels, since these are drawn on a 24 unit grid. */
  size?: number;
};

function Icon({ size = 16, children, ...rest }: IconProps & { children: ReactNode }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.9}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      {...rest}
    >
      {children}
    </svg>
  );
}

export function FolderIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z" />
    </Icon>
  );
}

export function SearchIcon(props: IconProps) {
  return (
    <Icon {...props} strokeWidth={2.2}>
      <circle cx="11" cy="11" r="7" />
      <path d="m20 20-3.5-3.5" />
    </Icon>
  );
}

export function SettingsIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <circle cx="12" cy="12" r="3.2" />
      <path d="M19.9 14.6a1.7 1.7 0 0 0 .34 1.87l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.7 1.7 0 0 0-2.87 1.15V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-2.87-1.15l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.7 1.7 0 0 0 3.1 14.6H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.15-2.87l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.7 1.7 0 0 0 9.95 3.1V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 2.87 1.15l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.7 1.7 0 0 0 1.15 2.87H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1 .65Z" />
    </Icon>
  );
}

export function CheckIcon(props: IconProps) {
  return (
    <Icon {...props} strokeWidth={2.6}>
      <path d="M20 6 9 17l-5-5" />
    </Icon>
  );
}

export function AlertIcon(props: IconProps) {
  return (
    <Icon {...props} strokeWidth={2.4}>
      <path d="M12 8v5M12 16.5v.5" />
    </Icon>
  );
}

/** The turned arrow that points from a film to the subtitle file beside it. */
export function PairedIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M4 5v10a2 2 0 0 0 2 2h13" />
      <path d="m16 14 3 3-3 3" />
    </Icon>
  );
}

export function DropIcon(props: IconProps) {
  return (
    <Icon {...props} strokeWidth={1.7}>
      <path d="M12 16V4M12 4 7 9M12 4l5 5" />
      <path d="M4 16v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2" />
    </Icon>
  );
}

/*
 * The playback controls. Play and pause are the two that are drawn filled
 * rather than stroked: they are the largest control in the bar and the only
 * one somebody looks for without reading, so they need to be a shape and not
 * an outline.
 */

export function PlayIcon(props: IconProps) {
  return (
    <Icon {...props} fill="currentColor" strokeWidth={1.4}>
      <path d="M8 5.2 19 12 8 18.8Z" />
    </Icon>
  );
}

export function PauseIcon(props: IconProps) {
  return (
    <Icon {...props} fill="currentColor" stroke="none">
      <rect x="7" y="5" width="3.6" height="14" rx="1.2" />
      <rect x="13.4" y="5" width="3.6" height="14" rx="1.2" />
    </Icon>
  );
}

/** The arrow that goes back, with the number of seconds inside the curve. */
export function SkipBackIcon(props: IconProps) {
  return (
    <Icon {...props} strokeWidth={1.8}>
      <path d="M11.4 5.4a7 7 0 1 1-6.3 4" />
      <path d="M4.4 2.4v3.6h3.6" />
    </Icon>
  );
}

export function SkipForwardIcon(props: IconProps) {
  return (
    <Icon {...props} strokeWidth={1.8}>
      <path d="M12.6 5.4a7 7 0 1 0 6.3 4" />
      <path d="M19.6 2.4v3.6H16" />
    </Icon>
  );
}

export function VolumeIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M4 9.5h3L11 6v12L7 14.5H4Z" />
      <path d="M14.6 9.4a3.6 3.6 0 0 1 0 5.2" />
      <path d="M17.4 6.8a7.2 7.2 0 0 1 0 10.4" />
    </Icon>
  );
}

export function MuteIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M4 9.5h3L11 6v12L7 14.5H4Z" />
      <path d="m15 10 4.5 4.5M19.5 10 15 14.5" />
    </Icon>
  );
}

/**
 * The transcript: the picture with the dialogue beside it, which is what the
 * button it sits on does to the screen.
 */
export function TranscriptIcon(props: IconProps) {
  return (
    <Icon {...props} strokeWidth={1.7}>
      <rect x="3" y="5" width="18" height="14" rx="2.5" />
      <path d="M13.5 5v14M16 9.5h2.5M16 14.5h2" />
    </Icon>
  );
}

export function FullscreenIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M4 9V5.5A1.5 1.5 0 0 1 5.5 4H9M15 4h3.5A1.5 1.5 0 0 1 20 5.5V9M20 15v3.5a1.5 1.5 0 0 1-1.5 1.5H15M9 20H5.5A1.5 1.5 0 0 1 4 18.5V15" />
    </Icon>
  );
}

export function ExitFullscreenIcon(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M9 4v3.5A1.5 1.5 0 0 1 7.5 9H4M20 9h-3.5A1.5 1.5 0 0 1 15 7.5V4M15 20v-3.5a1.5 1.5 0 0 1 1.5-1.5H20M4 15h3.5A1.5 1.5 0 0 1 9 16.5V20" />
    </Icon>
  );
}

export function MinimiseIcon(props: IconProps) {
  return (
    <Icon {...props} viewBox="0 0 12 12" strokeWidth={1.1}>
      <path d="M1 6h10" />
    </Icon>
  );
}

export function MaximiseIcon(props: IconProps) {
  return (
    <Icon {...props} viewBox="0 0 12 12" strokeWidth={1.1}>
      <rect x="1.5" y="1.5" width="9" height="9" />
    </Icon>
  );
}

export function RestoreIcon(props: IconProps) {
  return (
    <Icon {...props} viewBox="0 0 12 12" strokeWidth={1.1}>
      <rect x="1.5" y="3.5" width="7" height="7" />
      <path d="M3.5 3.5v-2h7v7h-2" />
    </Icon>
  );
}

export function CloseIcon(props: IconProps) {
  return (
    <Icon {...props} viewBox="0 0 12 12" strokeWidth={1.1}>
      <path d="M1.5 1.5l9 9M10.5 1.5l-9 9" />
    </Icon>
  );
}
