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
    await page.setViewportSize({ width: 375, height: 667 })
    const sidebarToggle = page.getByLabel('فتح الشريط الجانبي')
    await expect(sidebarToggle).toBeVisible()
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
    await page.goto('/')
    const html = page.locator('html')
    await expect(page.getByRole('banner')).toBeVisible()

    // Open the theme dropdown menu and select dark mode
    await page.getByLabel('الوضع الداكن').click()
    await page.getByRole('button', { name: 'داكن', exact: true }).click()

    await expect(html).toHaveClass(/dark/)
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
    await page.goto('/')
    const chatTab = page
      .getByRole('navigation', { name: 'القائمة الرئيسية' })
      .getByRole('tab', { name: 'الشات' })
    await expect(chatTab).toBeVisible()
    // WebKit does not grant a freshly loaded document keyboard focus until a
    // user gesture; click a real focusable to enter the focus chain, then Tab
    // must advance focus to the next element.
    await chatTab.click()
    const focusBefore = await page.evaluate(
      () => `${document.activeElement?.tagName}.${document.activeElement?.className}`,
    )
    await page.keyboard.press('Tab')
    const focusAfter = await page.evaluate(
      () => `${document.activeElement?.tagName}.${document.activeElement?.className}`,
    )
    expect(focusAfter).not.toBe(focusBefore)
    expect(focusAfter).not.toMatch(/^BODY/)
  })
})
