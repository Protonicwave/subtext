import { type ReactNode, useId } from 'react';
import type { AmountName, SettingName, Settings, ToggleName } from '@/shared/settings/schema';
import { rangeOf } from '@/shared/settings/schema';
import { useSettings } from '@/shared/settings/useSettings';
import styles from './controls.module.css';

/**
 * The three controls the settings screen is made of.
 *
 * Each one is bound to a setting by name rather than handed a value and a
 * callback, because every one of them does the same two things with them and
 * repeating that twenty times is twenty chances to wire one up to nothing.
 *
 * All three are built on the platform's own inputs. A radio group already
 * answers to the arrow keys and announces which of a set is chosen, a checkbox
 * already toggles on space, and a range already moves by its own step: none of
 * that is worth writing again on top of a div.
 */

interface Described {
  label: string;
  /** The line under it. What the choice means, not what the words mean. */
  hint?: ReactNode;
}

interface ChoiceProps<Name extends SettingName> extends Described {
  name: Name;
  options: readonly { value: Settings[Name]; label: string }[];
}

/** One of a few named alternatives, drawn as a strip. */
export function Choice<Name extends SettingName>({
  name,
  label,
  hint,
  options,
}: ChoiceProps<Name>) {
  const chosen = useSettings((state) => state.settings[name]);
  const change = useSettings((state) => state.change);
  // Two groups on one screen must not answer to each other's arrow keys.
  const group = useId();

  return (
    <fieldset className={styles.row}>
      <legend className={styles.text}>
        <b>{label}</b>
        {hint !== undefined && <small>{hint}</small>}
      </legend>

      <div className={styles.strip}>
        {options.map((option) => (
          <label key={String(option.value)} className={styles.option}>
            <input
              type="radio"
              name={group}
              className={styles.hidden}
              checked={option.value === chosen}
              onChange={() => {
                change(name, option.value);
              }}
            />
            <span>{option.label}</span>
          </label>
        ))}
      </div>
    </fieldset>
  );
}

/**
 * One of many named alternatives, drawn as a list.
 *
 * The same thing [`Choice`] is, for a set too long to lay out as a strip. Thirty
 * languages side by side would be a wall; a select is one line and the platform
 * draws the list, which on every platform is a better list than one built here.
 */
export function Picker<Name extends SettingName>({
  name,
  label,
  hint,
  options,
}: ChoiceProps<Name>) {
  const chosen = useSettings((state) => state.settings[name]);
  const change = useSettings((state) => state.change);
  const field = useId();

  return (
    <div className={styles.row}>
      <label className={styles.text} htmlFor={field}>
        <b>{label}</b>
        {hint !== undefined && <small>{hint}</small>}
      </label>

      <select
        id={field}
        className={styles.picker}
        value={String(chosen)}
        onChange={(event) => {
          const picked = options.find((option) => String(option.value) === event.target.value);
          if (picked !== undefined) change(name, picked.value);
        }}
      >
        {options.map((option) => (
          <option key={String(option.value)} value={String(option.value)}>
            {option.label}
          </option>
        ))}
      </select>
    </div>
  );
}

/** Something that is either done or not. */
export function Switch({ name, label, hint }: Described & { name: ToggleName }) {
  const on = useSettings((state) => state.settings[name]);
  const change = useSettings((state) => state.change);

  return (
    <label className={styles.row}>
      <span className={styles.text}>
        <b>{label}</b>
        {hint !== undefined && <small>{hint}</small>}
      </span>

      <input
        type="checkbox"
        role="switch"
        className={styles.switch}
        checked={on}
        onChange={(event) => {
          change(name, event.target.checked);
        }}
      />
    </label>
  );
}

interface SliderProps extends Described {
  name: AmountName;
  /** The value said in the units a person thinks in, rather than the stored one. */
  format: (value: number) => string;
}

/** A number between two ends, with what it currently says beside it. */
export function Slider({ name, label, hint, format }: SliderProps) {
  const value = useSettings((state) => state.settings[name]);
  const change = useSettings((state) => state.change);
  const range = rangeOf(name);
  const field = useId();

  return (
    <div className={styles.row}>
      <label className={styles.text} htmlFor={field}>
        <b>{label}</b>
        {hint !== undefined && <small>{hint}</small>}
      </label>

      <div className={styles.slider}>
        <output className={styles.reading} htmlFor={field}>
          {format(value)}
        </output>
        <input
          id={field}
          type="range"
          min={range.least}
          max={range.most}
          step={range.step}
          value={value}
          onChange={(event) => {
            change(name, event.target.valueAsNumber);
          }}
        />
      </div>
    </div>
  );
}

/** A row that is an action rather than a preference. */
export function Action({ label, hint, children }: Described & { children: ReactNode }): ReactNode {
  return (
    <div className={styles.row}>
      <span className={styles.text}>
        <b>{label}</b>
        {hint !== undefined && <small>{hint}</small>}
      </span>
      {children}
    </div>
  );
}
