using System;
using System.Linq;
using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;

namespace AircastQGC.Controls;

public sealed partial class NavPane : UserControl
{
    public const string Fly = "Fly";
    public const string Plan = "Plan";
    public const string Analyze = "Analyze";
    public const string VehicleSetup = "Vehicle Setup";
    public const string Settings = "Settings";

    private const double OpenWidth = 280;
    private const double CompactWidth = 48;

    private string _selectedPage = Fly;
    private bool _listPaneOpen = true;
    private bool _mapPaneOpen;

    public NavPane()
    {
        InitializeComponent();
        Render();
    }

    public event EventHandler<string>? NavigationRequested;

    public event EventHandler? BackRequested;

    public string SelectedPage
    {
        get => _selectedPage;
        set
        {
            _selectedPage = value;
            Render();
        }
    }

    public bool IsPaneOpen
    {
        get => OnMap ? _mapPaneOpen : _listPaneOpen;
        set
        {
            if (OnMap) _mapPaneOpen = value;
            else _listPaneOpen = value;
            Render();
        }
    }

    private bool OnMap => _selectedPage == Plan;

    public bool CanGoBack
    {
        get => BackButton.IsEnabled;
        set => BackButton.IsEnabled = value;
    }

    private Button[] Items => new[] { FlyItem, PlanItem, AnalyzeItem, SetupItem, SettingsItem };

    private void OnNavigate(object sender, RoutedEventArgs e)
    {
        if (sender is not Button { Tag: string page }) return;
        NavigationRequested?.Invoke(this, page);
    }

    private void OnBack(object sender, RoutedEventArgs e) => BackRequested?.Invoke(this, EventArgs.Empty);

    private void Render()
    {
        Root.Width = IsPaneOpen ? OpenWidth : CompactWidth;
        Array.ForEach(Items, item => Paint(item, item.Tag as string == _selectedPage, IsPaneOpen));
    }

    private static void Paint(Button item, bool selected, bool open)
    {
        var parts = ((Panel)item.Content).Children;
        var label = parts.OfType<TextBlock>().First();
        parts.OfType<Border>().First().Opacity = selected ? 1 : 0;
        parts.OfType<FontIcon>().First().Foreground = Token(selected ? "AccentBrush" : "TextSecondaryBrush");
        label.Foreground = Token(selected ? "TextPrimaryBrush" : "TextSecondaryBrush");
        label.Visibility = open ? Visibility.Visible : Visibility.Collapsed;
        item.Background = Token(selected ? "SubtleSelectedBrush" : "");
    }

    private static Brush Token(string key) =>
        key.Length > 0 && Application.Current.Resources.TryGetValue(key, out var value) && value is Brush brush
            ? brush
            : new SolidColorBrush(Colors.Transparent);
}
