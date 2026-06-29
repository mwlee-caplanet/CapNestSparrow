using System;
using System.Collections.Generic;
using System.Text;
using Windows.UI.ViewManagement;
using Microsoft.Win32;
using System.Windows;

namespace CAPlanetSparrowSample.WPF.Services
{
    public class CommonDialogService
    {
        public static string? OpenSparrowInputFileDialog()
        {
            var dialog = new OpenFileDialog
            {
                DefaultExt = ".json",
                Filter = $"Sparrow Input Data File|*.json",
                InitialDirectory = AppDomain.CurrentDomain.BaseDirectory,
                Multiselect = false
            };
            if (dialog.ShowDialog(Application.Current.MainWindow) == false)
            {
                return null;
            }

            return dialog.FileName;
        }
    }
}
