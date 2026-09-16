import * as React from "react";

import { Input } from "@/components/ui/input";

type NumberInputProps = Omit<React.ComponentPropsWithoutRef<"input">, "type" | "onChange" | "onWheel" | "min"> & {
  /** Called with a digits-only string; existing `e.target.value` handlers keep working. */
  onChange?: React.ChangeEventHandler<HTMLInputElement>;
  /** Values below this clamp up on blur. */
  min?: number;
  /** Allow decimal point (default false — whole numbers only). */
  decimal?: boolean;
};

/**
 * Numeric entry used for every quantity/money field in the app.
 *
 * - Mouse wheel over the field never changes the value (blurs instead, so the
 *   page keeps scrolling normally). Spin buttons are hidden globally in
 *   styles.css.
 * - Digits only: whole rupees and whole units are canonical across the system
 *   (see HANDOFF locked decisions), so "-", "e", "+" etc. are stripped as
 *   they are typed — a stray minus can never inflate or deflate an amount.
 * - On blur, empty stays empty and values below `min` clamp up to `min`.
 */
const NumberInput = React.forwardRef<HTMLInputElement, NumberInputProps>(
  ({ value, defaultValue, onChange, onBlur, min = 0, decimal = false, ...props }, ref) => {
    const sanitize = (raw: string) => decimal ? raw.replace(/[^0-9.]/g, "") : raw.replace(/[^0-9]/g, "");

    const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
      const cleaned = sanitize(e.target.value);
      if (cleaned !== e.target.value) e.target.value = cleaned;
      onChange?.(e);
    };

    const handleBlur = (e: React.FocusEvent<HTMLInputElement>) => {
      let cleaned = sanitize(e.target.value);
      if (cleaned !== "" && Number(cleaned) < min) cleaned = String(min);
      if (cleaned !== e.target.value) {
        e.target.value = cleaned;
        onChange?.({ ...e, target: e.target, currentTarget: e.target } as React.ChangeEvent<HTMLInputElement>);
      }
      onBlur?.(e);
    };

    return (
      <Input
        ref={ref}
        type="number"
        inputMode={decimal ? "decimal" : "numeric"}
        min={min}
        value={value}
        defaultValue={defaultValue}
        onChange={handleChange}
        onBlur={handleBlur}
        onWheel={(e) => {
          // Only a focused number field mutates on wheel; blur it so the page scrolls.
          if (document.activeElement === e.currentTarget) e.currentTarget.blur();
        }}
        {...props}
      />
    );
  },
);
NumberInput.displayName = "NumberInput";

export { NumberInput };
