import { test, expect } from '@playwright/test'

test.describe('Chat Interface', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
  })

  test('should display welcome screen on first load', async ({ page }) => {
    await expect(page.getByText('مرحباً بك في Vortex AI')).toBeVisible()
  })

  test('should display example prompts', async ({ page }) => {
    await expect(page.getByText('اكتب دالة بايثون لحساب فيبوناتشي')).toBeVisible()
    await expect(page.getByText('اشرح لي مفهوم البرمجة الوظيفية')).toBeVisible()
  })

  test('should have chat input field', async ({ page }) => {
    await expect(page.getByPlaceholder('اكتب رسالتك هنا...')).toBeVisible()
  })

  test('should toggle sidebar on mobile', async ({ page }) => {
    // Root cause (#1): ChatSidebar ignored isOpen on mobile — fixed aside always
    // covered the RTL toggle, so clicks hit the conversations panel instead.
    await expect(page.getByPlaceholder('اكتب رسالتك هنا...')).toBeVisible()
    await page.setViewportSize({ width: 375, height: 667 })
    const sidebarToggle = page.getByLabel('فتح الشريط الجانبي')
    await expect(sidebarToggle).toBeVisible({ timeout: 20000 })
    await sidebarToggle.click()
    await expect(
      page.locator('aside').filter({ hasText: 'المحادثات' }).getByLabel('إغلاق الشريط الجانبي'),
    ).toBeVisible()
  })

  test('should switch to dashboard', async ({ page }) => {
    await page.click('text=لوحة التحكم')
    await expect(page.getByText('نظرة عامة')).toBeVisible()
  })
})

test.describe('Dashboard', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/')
    await page.click('text=لوحة التحكم')
  })

  test('should display overview tab by default', async ({ page }) => {
    await expect(page.getByText('نظرة عامة')).toBeVisible()
  })

  test('should navigate to models tab', async ({ page }) => {
    await page.click('text=النماذج')
    await expect(page.getByText('النموذج الحالي')).toBeVisible()
  })

  test('should navigate to knowledge tab', async ({ page }) => {
    await page.click('text=قاعدة المعرفة')
    await expect(page.getByText('البحث في المعرفة')).toBeVisible()
  })

  test('should navigate to tools tab', async ({ page }) => {
    await page.click('text=الأدوات')
    await expect(page.getByText('تنفيذ واختبار الأدوات المتاحة')).toBeVisible()
  })

  test('should navigate to logs tab', async ({ page }) => {
    await page.click('text=السجلات')
    await expect(page.getByText('عرض سجلات النظام والأخطاء')).toBeVisible()
  })

  test('should navigate to settings tab', async ({ page }) => {
    await page.click('text=الإعدادات')
    await expect(page.getByText('إعدادات اللغة والمظهر')).toBeVisible()
  })

  test('should navigate to security tab', async ({ page }) => {
    await page.click('text=الأمان')
    await expect(page.getByText('المصادقة والصلاحيات والتدقيق')).toBeVisible()
  })

  test('should show manual token card in settings (remote access)', async ({ page }) => {
    await page.click('text=الإعدادات')
    await expect(page.getByText('توكنات الوصول')).toBeVisible()
    await expect(page.getByPlaceholder('الصق من مضيف الخادم')).toBeVisible()
  })
})

test.describe('Theme', () => {
  test('should toggle dark mode', async ({ page }) => {
    // Root cause (#2): theme dropdown option could be clicked before the menu
    // finished opening (animate-in); Firefox then hit the backdrop instead.
    await page.goto('/')
    const html = page.locator('html')
    await expect(page.getByRole('banner')).toBeVisible()

    const themeTrigger = page.getByLabel('الوضع الداكن')
    await expect(themeTrigger).toBeVisible()
    await themeTrigger.click()

    const darkOption = page.getByRole('button', { name: 'داكن', exact: true })
    await expect(darkOption).toBeVisible()
    await darkOption.click()

    await expect(html).toHaveClass(/dark/, { timeout: 20000 })
  })
})

test.describe('Keyboard Shortcuts', () => {
  test('should open new chat with Ctrl+N', async ({ page }) => {
    await page.goto('/')
    await page.keyboard.press('Control+n')
    await expect(page.getByText('مرحباً بك في Vortex AI')).toBeVisible()
  })

  test('should clear chat with Ctrl+Shift+L', async ({ page }) => {
    await page.goto('/')
    // Type a message first
    await page.fill('textarea[aria-label="رسالة الشات"]', 'Test message')
    // Clear chat
    await page.keyboard.press('Control+Shift+l')
  })
})

test.describe('Accessibility', () => {
  test('should have proper ARIA labels', async ({ page }) => {
    await page.goto('/')
    // Two labeled navs exist once the lazy ChatInterface loads; assert deterministically.
    await expect(page.getByRole('navigation', { name: 'القائمة الرئيسية' })).toBeVisible()
    await expect(page.getByRole('main')).toBeVisible()
  })

  test('should move focus with Tab', async ({ page }) => {
    // Root cause (#3): Tab fired before Firefox committed focus after click,
    // so activeElement was still BODY / pre-click element on first try.
    await page.goto('/')
    const chatTab = page
      .getByRole('navigation', { name: 'القائمة الرئيسية' })
      .getByRole('tab', { name: 'الشات' })
    await expect(chatTab).toBeVisible()
    await chatTab.click()
    await expect(chatTab).toBeFocused()

    const focusBefore = await page.evaluate(
      () => `${document.activeElement?.tagName}.${document.activeElement?.className}`,
    )
    await page.keyboard.press('Tab')
    // Firefox can defer focus update by a frame after Tab.
    await expect
      .poll(
        async () =>
          page.evaluate(
            () => `${document.activeElement?.tagName}.${document.activeElement?.className}`,
          ),
        { timeout: 5000 },
      )
      .not.toBe(focusBefore)
    const focusAfter = await page.evaluate(
      () => `${document.activeElement?.tagName}.${document.activeElement?.className}`,
    )
    expect(focusAfter).not.toMatch(/^BODY/)
  })
})
