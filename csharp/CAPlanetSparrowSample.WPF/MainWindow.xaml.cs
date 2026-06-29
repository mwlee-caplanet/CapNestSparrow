using System.Text;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Data;
using System.Windows.Documents;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Navigation;
using System.Windows.Shapes;
using ReactiveUI;
using ReactiveUI.SourceGenerators;

namespace CAPlanetSparrowSample.WPF
{
    [IViewFor<MainWindowViewModel>]
    public partial class MainWindow
    {
        public MainWindow(MainWindowViewModel VM)
        {
            InitializeComponent();
            this.DataContext = VM;
        }
    }
}