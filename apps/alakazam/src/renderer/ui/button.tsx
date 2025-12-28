import { cn } from '@/lib/util';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faChevronDown, faChevronUp } from '@fortawesome/sharp-light-svg-icons';
import { cva } from 'class-variance-authority';
import React, {
  Children,
  ComponentProps,
  MouseEvent,
  ReactElement,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { createPortal } from 'react-dom';

export interface ButtonProps {
  children: React.ReactNode;
  size?: 'sm' | 'md' | 'lg';
  variant?: 'primary' | 'secondary' | 'tertiary' | 'ghost' | 'ghost-hover' | 'icon';
  active?: boolean;
  className?: string;
  onClick?: (e: MouseEvent<HTMLButtonElement>) => void;
  onOptionsOpenChange?: (open: boolean) => void;
  /**
   * Defines the aspect ratio of the button.
   * - `square`: The button will have equal width and height.
   * - `auto`: The button's width and height will adjust based on its content. Default behavior.
   */
  aspect?: 'square' | 'auto';
}

const buttonVariants = cva(
  'flex rounded-md font-medium focus:outline-none disabled:opacity-50 disabled:pointer-events-none cursor-pointer w-full items-center',
  {
    variants: {
      variant: {
        primary: 'bg-gray-600 text-white hover:bg-gray-700',
        secondary: 'bg-neutral-500 text-white hover:bg-neutral-500/60',
        tertiary: 'bg-transparent text-blue-600 hover:bg-blue-100',
        ghost: 'bg-transparent text-white hover:bg-transparent',
        'ghost-hover': 'bg-transparent text-white hover:bg-white/10',
        icon: 'bg-transparent text-white hover:bg-white/10 w-auto justify-center items-center',
      },
      size: {
        sm: 'px-2.5 py-1.5 text-sm',
        md: 'px-4 py-2 text-base',
        lg: 'px-6 py-3 text-lg',
      },
      active: {
        true: 'bg-white/20',
        false: '',
      },
      aspect: {
        square: 'aspect-square',
        auto: '',
      },
    },
    compoundVariants: [
      {
        variant: 'icon',
        size: 'sm',
        className: 'p-1 text-sm',
      },
      {
        variant: 'icon',
        size: 'md',
        className: 'p-1 text-md',
      },
      {
        variant: 'icon',
        size: 'lg',
        className: 'p-2 text-lg',
      },
    ],
    defaultVariants: {
      variant: 'primary',
      size: 'md',
    },
  },
);
/**
 * A container for button options used in conjunction with the `Button` component.
 * - `placement`: Determines where the options popup appears relative to the button. Can be 'above' or 'below'. Default is 'below'.
 * - `activationStyle`: Defines how the options are activated. Can be 'side' Clicking on the secondary part of the button activates the options, or 'whole' Clicking anywhere on the button activates the options. Default is 'side'.
 * - `onOptionSelected`: A callback function that is invoked when an option is selected, receiving the selected option's value as an argument.
 * @remarks
 * This component does not render anything by itself. It is used to define options
 * that can be selected when the button is clicked.
 */
export function ButtonOptions({
  children,
  placement = 'below',
  activationStyle = 'side',
  onOptionSelected,
}: {
  children: React.ReactNode;
  placement?: 'above' | 'below';
  activationStyle?: 'side' | 'whole';
  onOptionSelected?: (optionValue: any) => void;
}) {
  return null;
}
ButtonOptions.displayName = 'ButtonOptions';

export function Button(props: ButtonProps & ComponentProps<'button'>) {
  const [hasOptions, setHasOptions] = useState(false);
  const [optionsOpen, setOptionsOpen] = useState(false);
  const splitButtonOptionsRef = useRef<HTMLButtonElement>(null);
  const popupRef = useRef<HTMLDivElement>(null);
  const [popupStyle, setPopupStyle] = useState<React.CSSProperties>({ position: 'fixed', left: -9999, top: -9999 });

  const buttonOptionsItem = useMemo(
    () =>
      Children.toArray(props.children).find(
        child => React.isValidElement(child) && (child.type as any).displayName === 'ButtonOptions',
      ) as ReactElement | undefined,
    [props.children],
  );

  const item = buttonOptionsItem as ReactElement<ComponentProps<typeof ButtonOptions>>;
  const placement = item?.props.placement ?? 'below';
  const activationStyle = item?.props.activationStyle ?? 'side';

  useEffect(() => {
    setHasOptions(!!buttonOptionsItem);
  }, [buttonOptionsItem]);

  useEffect(() => {
    props.onOptionsOpenChange?.(optionsOpen);
  }, [optionsOpen, props.onOptionsOpenChange]);

  useLayoutEffect(() => {
    if (optionsOpen && popupRef.current && splitButtonOptionsRef.current && buttonOptionsItem) {
      const buttonRect = splitButtonOptionsRef.current.getBoundingClientRect();
      const popupRect = popupRef.current.getBoundingClientRect();

      const centerX = buttonRect.left + buttonRect.width / 2;
      let top: number;
      let left = centerX - popupRect.width / 2;
      const additionalOffset = 8; // Small offset to avoid touching the button screen edges

      // Adjust vertical position
      if (placement === 'above') {
        top = buttonRect.top - popupRect.height - additionalOffset;
        if (top < 0) {
          // Off screen above, move below
          top = buttonRect.bottom + additionalOffset;
        }
      } else {
        top = buttonRect.bottom + additionalOffset;
        if (top + popupRect.height > window.innerHeight) {
          // Off screen below, move above
          top = buttonRect.top - popupRect.height - additionalOffset;
        }
      }

      // Adjust horizontal position
      if (left < 0) {
        left = additionalOffset;
      } else if (left + popupRect.width > window.innerWidth) {
        left = window.innerWidth - popupRect.width - additionalOffset;
      }

      setPopupStyle({ left, top, position: 'fixed' });
    } else if (!optionsOpen) {
      setPopupStyle({ position: 'fixed', left: -9999, top: -9999 });
    }
  }, [optionsOpen, buttonOptionsItem]);

  useEffect(() => {
    if (!optionsOpen) return;

    const handleClickOutside = (event: Event) => {
      if (
        popupRef.current &&
        !popupRef.current.contains(event.target as Node) &&
        splitButtonOptionsRef.current &&
        !splitButtonOptionsRef.current.contains(event.target as Node)
      ) {
        setOptionsOpen(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [optionsOpen]);

  const baseButton = (extraClassName = '') => (
    <button
      {...props}
      onClick={e => {
        if (hasOptions && activationStyle === 'whole') setOptionsOpen(!optionsOpen);
        props.onClick?.(e);
      }}
      className={cn(
        buttonVariants({ variant: props.variant, size: props.size, aspect: props.aspect, active: props.active }),
        props.className,
        extraClassName,
      )}
    >
      {props.children}
    </button>
  );

  const splitButton = () => (
    <div className="inline-flex rounded-md p-1 hover:bg-white/10" data-test="split-button">
      {baseButton('hover:bg-transparent p-0')}
      {hasOptions && (
        <button
          className="cursor-pointer items-center justify-center"
          onClick={() => setOptionsOpen(!optionsOpen)}
          ref={splitButtonOptionsRef}
        >
          {optionsOpen ? (
            <FontAwesomeIcon icon={faChevronUp} size="sm" />
          ) : (
            <FontAwesomeIcon icon={faChevronDown} size="sm" />
          )}
        </button>
      )}
    </div>
  );

  const buttonOptions = () => {
    if (!buttonOptionsItem) return null;

    const item = buttonOptionsItem as ReactElement<{
      placement?: 'above' | 'below';
      children: React.ReactNode;
      onOptionSelected?: <T>(optionValue: T) => void;
    }>;
    const options = Children.toArray(item.props.children)
      .filter(
        (child): child is ReactElement<{ value: unknown }> =>
          React.isValidElement(child) && (child.type as any).displayName === 'Option',
      )
      .map((child, index) => (
        <div
          key={index}
          role="menuitem"
          className="px-3 py-2 whitespace-nowrap hover:bg-gray-700"
          onClick={() => {
            const optionValue = child.props.value;
            item.props.onOptionSelected?.(optionValue);
            setOptionsOpen(false);
          }}
        >
          {(child.props as any).children}
        </div>
      ));

    return (
      <div
        ref={popupRef}
        className="bg-dark absolute text-white"
        style={popupStyle}
        role="menu"
        aria-orientation="vertical"
        aria-labelledby="options-menu"
      >
        {options}
      </div>
    );
  };

  return (
    <>
      {hasOptions ? splitButton() : baseButton()}
      {optionsOpen && createPortal(buttonOptions(), document.body)}
    </>
  );
}
Button.displayName = 'Button';
