import { MenuItem, menubarItems as menuItems } from '@/client/data/menu-bar-items';
import { cn } from '@/client/lib/util';
import { FontAwesomeIcon, FontAwesomeLayers } from '@fortawesome/react-fontawesome';
import { faAngleRight, faSquare as faSquareFull } from '@fortawesome/sharp-duotone-light-svg-icons';
import { faSquare, faTimes, faWindowMinimize } from '@fortawesome/sharp-light-svg-icons';
import React, { createContext, useContext, useEffect, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';

const MenubarContext = createContext({
  openMenu: null as string | null,
  setOpenMenu: (menu: string | null) => {},
});

function TitleBarMenuItem({ item }: { item: MenuItem }) {
  const [openSubmenu, setOpenSubmenu] = useState<string | null>(null);
  const [openSubmenuPosition, setOpenSubmenuPosition] = useState<{ x: number; y: number }>({ x: 0, y: 0 });
  const menuTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const closeTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const divRef = useRef<HTMLDivElement>(null);
  const submenuRef = useRef<HTMLDivElement>(null);
  const { setOpenMenu } = useContext(MenubarContext);

  useLayoutEffect(() => {
    if (openSubmenu === item.label && submenuRef.current) {
      const rect = submenuRef.current.getBoundingClientRect();
      let newX = openSubmenuPosition.x;
      let newY = openSubmenuPosition.y;
      const additionalOffset = 8;

      if (newX + rect.width > window.innerWidth) {
        newX = window.innerWidth - rect.width - additionalOffset;
      }
      if (newX < 0) {
        newX = additionalOffset;
      }
      if (newY + rect.height > window.innerHeight) {
        newY = window.innerHeight - rect.height - additionalOffset;
      }
      if (newY < 0) {
        newY = additionalOffset;
      }

      if (newX !== openSubmenuPosition.x || newY !== openSubmenuPosition.y) {
        setOpenSubmenuPosition({ x: newX, y: newY });
      }
    }
  }, [openSubmenu, item.label, openSubmenuPosition]);

  const handleOpenSubmenu = (event: React.MouseEvent) => {
    if (menuTimeoutRef.current) {
      clearTimeout(menuTimeoutRef.current);
      menuTimeoutRef.current = null;
    }
    if (closeTimeoutRef.current) {
      clearTimeout(closeTimeoutRef.current);
      closeTimeoutRef.current = null;
    }
    const { left, top, width } = event.currentTarget.getBoundingClientRect();
    const timeout = setTimeout(() => {
      let x = left + width;
      let y = top;
      setOpenSubmenuPosition({ x, y });
      setOpenSubmenu(item.label ?? '');
    }, 500);
    menuTimeoutRef.current = timeout;
  };

  const handleCloseSubmenu = (event: React.MouseEvent) => {
    if (menuTimeoutRef.current) {
      clearTimeout(menuTimeoutRef.current);
      menuTimeoutRef.current = null;
    }
    if (closeTimeoutRef.current) {
      clearTimeout(closeTimeoutRef.current);
      closeTimeoutRef.current = null;
    }
    // If the mouse leaves the parent menu item (for example, to move into
    // the submenu), set a short timeout to close the submenu. The submenu's
    // `onEnter` will clear this timeout when the pointer enters it so it
    // doesn't immediately close while the user is moving the mouse.
    const timeout = setTimeout(() => {
      setOpenSubmenu(null);
      closeTimeoutRef.current = null;
    }, 150);
    closeTimeoutRef.current = timeout;
  };

  const handleOnMenuAction = (action?: () => void) => {
    if (action) action();
    setOpenSubmenu(null);
    setOpenMenu(null);
  };

  useEffect(() => {
    return () => {
      if (menuTimeoutRef.current) clearTimeout(menuTimeoutRef.current);
      if (closeTimeoutRef.current) clearTimeout(closeTimeoutRef.current);
    };
  }, []);

  return (
    <div data-label={item.label} className="text-active" ref={divRef}>
      {item.type === 'separator' ? (
        <div className="my-2 border border-neutral-50/20" />
      ) : (
        <div
          className={cn('flex items-center justify-between px-3 py-2 hover:bg-gray-700', {
            'pointer-events-none opacity-50': item.disabled,
          })}
          onMouseEnter={handleOpenSubmenu}
          onMouseLeave={handleCloseSubmenu}
          onClick={() => handleOnMenuAction(item.click)}
          tabIndex={0}
          aria-disabled={item.disabled}
          onKeyDown={e => {
            if (e.key === 'Enter') {
              const el = e.currentTarget as HTMLElement;
              const { left, top, width } = el.getBoundingClientRect();
              // If submenu exists, open it and clear any close timeout.
              if (item.submenu && item.submenu.length > 0) {
                if (closeTimeoutRef.current) {
                  clearTimeout(closeTimeoutRef.current);
                  closeTimeoutRef.current = null;
                }
                let x = left + width;
                if (x + 200 > window.innerWidth) {
                  x = left - 200;
                }
                let y = top;
                if (y + 200 > window.innerHeight) {
                  y = window.innerHeight - 200;
                }
                setOpenSubmenuPosition({ x, y });
                setOpenSubmenu(item.label ?? '');
              }
            }
          }}
        >
          <div className="">{item.label}</div>
          {item.submenu && item.submenu.length > 0 && <FontAwesomeIcon icon={faAngleRight} />}
        </div>
      )}
      {item.submenu && (
        <>
          {divRef.current &&
            openSubmenu === item.label &&
            createPortal(
              <TitleBarDropdown
                ref={submenuRef}
                isRoot={false}
                x={openSubmenuPosition.x}
                y={openSubmenuPosition.y}
                menuItems={item.submenu}
                onEnter={() => {
                  if (closeTimeoutRef.current) {
                    clearTimeout(closeTimeoutRef.current);
                    closeTimeoutRef.current = null;
                  }
                }}
                onLeave={() => {
                  const timeout = setTimeout(() => {
                    setOpenSubmenu(null);
                    closeTimeoutRef.current = null;
                  }, 150);
                  closeTimeoutRef.current = timeout;
                }}
              />,
              document.body,
            )}
        </>
      )}
    </div>
  );
}

const TitleBarDropdown = React.forwardRef<
  HTMLDivElement,
  {
    isRoot?: boolean;
    menuItems?: MenuItem[];
    x?: number;
    y?: number;
    onEnter?: () => void;
    onLeave?: () => void;
  }
>(({ isRoot = true, menuItems, x, y, onEnter, onLeave }, ref) => {
  return (
    <div
      ref={ref}
      data-type="dropdown"
      className={cn('bg-dark absolute top-0 left-0 z-10 w-[200px] shadow-lg', {
        'mt-10': isRoot,
      })}
      style={{ left: x, top: y }}
      onMouseEnter={onEnter}
      onMouseLeave={onLeave}
    >
      {menuItems?.map((menuItem, index) => (
        <TitleBarMenuItem key={index} item={menuItem} />
      ))}
    </div>
  );
});
TitleBarDropdown.displayName = 'TitleBarDropdown';

function MenuItemRoot({
  menuItem,
  openMenu,
  openMenuPosition,
  handleMenuClick,
  setOpenMenuPosition,
  setOpenMenu,
}: {
  menuItem: MenuItem;
  openMenu: string | null;
  openMenuPosition: { x: number; y: number };
  handleMenuClick: (menuLabel?: string, event?: React.MouseEvent) => void;
  setOpenMenuPosition: (position: { x: number; y: number }) => void;
  setOpenMenu: (menu: string | null) => void;
}) {
  const dropdownRef = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    if (openMenu === menuItem.label && dropdownRef.current) {
      const rect = dropdownRef.current.getBoundingClientRect();
      let newX = openMenuPosition.x;
      let newY = openMenuPosition.y;
      const additionalOffset = 8;

      if (newX + rect.width > window.innerWidth) {
        newX = window.innerWidth - rect.width - additionalOffset;
      }
      if (newX < 0) {
        newX = additionalOffset;
      }
      if (newY + rect.height > window.innerHeight) {
        newY = window.innerHeight - rect.height - additionalOffset;
      }
      if (newY < 0) {
        newY = additionalOffset;
      }

      if (newX !== openMenuPosition.x || newY !== openMenuPosition.y) {
        setOpenMenuPosition({ x: newX, y: newY });
      }
    }
  }, [openMenu, menuItem.label, openMenuPosition]);

  return (
    <>
      <button
        data-menu-button
        className="px-3 py-2 hover:bg-gray-700"
        style={{ WebkitAppRegion: 'no-drag' } as any}
        onClick={e => handleMenuClick(menuItem.label, e)}
        onMouseEnter={(e: React.MouseEvent) => {
          // Only switch root menu on hover when a menu is currently open.
          if (!openMenu) return;
          if (openMenu === menuItem.label) return;
          const { left, top } = (e.currentTarget as HTMLElement).getBoundingClientRect();
          if (menuItem.submenu && menuItem.submenu.length > 0) {
            let x = left;
            let y = top;
            setOpenMenuPosition({ x, y });
            setOpenMenu(menuItem.label ?? '');
          } else {
            // If the hovered root item has no submenu, close any open menu.
            setOpenMenu(null);
          }
        }}
      >
        {menuItem.label}
      </button>
      <>
        {menuItem.label === openMenu &&
          createPortal(
            <TitleBarDropdown ref={dropdownRef} x={openMenuPosition.x} menuItems={menuItem.submenu} />,
            document.body,
          )}
      </>
    </>
  );
}

export function TitleBar() {
  const [isMaximized, setIsMaximized] = useState(false);
  const [openMenu, setOpenMenu] = useState<string | null>(null);
  const [openMenuPosition, setOpenMenuPosition] = useState<{ x: number; y: number }>({ x: 0, y: 0 });
  const [isDev, setIsDev] = useState(false);

  useEffect(() => {
    const checkMaximized = async () => {
      if (window.alakazam && window.alakazam.isMaximized) {
        const maximized = await window.alakazam.isMaximized();
        setIsMaximized(maximized);
      }
    };
    checkMaximized();
    window.alakazam.developer.isDev().then(setIsDev);
  }, []);

  const RestoreIcon = () => (
    <FontAwesomeLayers>
      <FontAwesomeIcon icon={faSquare} transform="right-2 up-2" />
      <FontAwesomeIcon
        icon={faSquareFull}
        transform="left-2 down-2"
        style={{
          '--fa-secondary-opacity': '1',
        }}
      />
    </FontAwesomeLayers>
  );

  const handleMenuClick = (menuLabel?: string, event?: React.MouseEvent) => {
    const { left, top } = event?.currentTarget.getBoundingClientRect() ?? {};
    let x = left ?? 0;
    let y = top ?? 0;
    setOpenMenuPosition({ x, y });
    setOpenMenu(menuLabel ?? null);
  };

  const handleMinimize = () => {
    if (window.alakazam && window.alakazam.minimizeWindow) {
      window.alakazam.minimizeWindow();
    }
  };

  const handleMaximize = async () => {
    if (window.alakazam && window.alakazam.maximizeWindow) {
      await window.alakazam.maximizeWindow();
      const maximized = await window.alakazam.isMaximized();
      setIsMaximized(maximized);
    }
  };

  const handleClose = () => {
    if (window.alakazam && window.alakazam.closeWindow) {
      window.alakazam.closeWindow();
    }
  };
  const handleDocumentClick = (event: MouseEvent) => {
    const target = event.target as HTMLElement;
    // If the menu is not open, allow clicks.
    if (!openMenu) return;
    // If the click is inside the open menu or on a menu button, do nothing.
    if (target.closest('[data-type="dropdown"]') || target.closest('[data-menu-button]')) return;
    // Otherwise, close the menu.
    setOpenMenu(null);
  };

  useEffect(() => {
    const unsubscribe = window.alakazam.onWindowLostFocus(() => setOpenMenu(null));
    if (openMenu) document.addEventListener('click', handleDocumentClick);
    else document.removeEventListener('click', handleDocumentClick);
    return () => {
      unsubscribe();
      document.removeEventListener('click', handleDocumentClick);
    };
  }, [openMenu]);

  return (
    <MenubarContext.Provider value={{ openMenu, setOpenMenu }}>
      <div
        className="bg-dark flex h-10 items-center justify-between text-white select-none"
        // style={{ WebkitAppRegion: 'drag' } as any}
      >
        <div className="flex">
          {menuItems.map((menuItem, index) => (
            <MenuItemRoot
              key={index}
              menuItem={menuItem}
              openMenu={openMenu}
              openMenuPosition={openMenuPosition}
              handleMenuClick={handleMenuClick}
              setOpenMenuPosition={setOpenMenuPosition}
              setOpenMenu={setOpenMenu}
            />
          ))}
        </div>
        <div className="flex">
          {isDev && (
            <MenuItemRoot
              menuItem={{
                label: 'Dev',
                submenu: [
                  { label: 'Refresh', click: () => window.location.reload() },
                  { label: 'Toggle DevTools', click: () => window.alakazam.developer.toggleDevTools() },
                ],
              }}
              openMenu={openMenu}
              openMenuPosition={openMenuPosition}
              handleMenuClick={handleMenuClick}
              setOpenMenuPosition={setOpenMenuPosition}
              setOpenMenu={setOpenMenu}
            />
          )}
          <button
            className="px-6 py-2 hover:bg-gray-600"
            style={{ WebkitAppRegion: 'no-drag' } as any}
            onClick={handleMinimize}
          >
            <FontAwesomeIcon icon={faWindowMinimize} />
          </button>
          <button
            className="px-6 py-2 [--fa-secondary-color:var(--color-dark)] hover:bg-gray-600 hover:[--fa-secondary-color:var(--color-gray-600)]"
            style={{ WebkitAppRegion: 'no-drag' } as any}
            onClick={handleMaximize}
          >
            {isMaximized ? <RestoreIcon /> : <FontAwesomeIcon icon={faSquare} />}
          </button>
          <button
            className="px-6 py-2 hover:bg-red-600"
            style={{ WebkitAppRegion: 'no-drag' } as any}
            onClick={handleClose}
          >
            <FontAwesomeIcon icon={faTimes} />
          </button>
        </div>
      </div>
    </MenubarContext.Provider>
  );
}
